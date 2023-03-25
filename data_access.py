import fnmatch
import json
import os
import polars as pl
from astropy import units as u
import astropy as ap
from astropy.coordinates import SkyCoord
from astropy.coordinates import Angle
import pyvo as vo
from pyvo import dal, registry

from typing import Callable, Iterable, Iterator, List, Tuple, Dict, Union, Optional
from concurrent.futures import ThreadPoolExecutor, as_completed
import time
from dataclasses import dataclass
import re


@dataclass
class FieldMetadata:
    name: str
    datatype: str
    description: str
    unit: u.Unit
    ucd: str
    arraysize: str
    xtype: str
    ref: str
    dtype: pl.DataType = None

    @classmethod
    def from_field(cls, field: ap.io.votable.tree.Field):
        return cls(
            name=field.name,
            datatype=field.datatype,
            description=field.description,
            unit=field.unit,
            ucd=field.ucd,
            arraysize=field.arraysize,
            xtype=field.xtype,
            ref=field.ref,
        )

    def to_polars_dtype(self) -> pl.DataType:
        if self.datatype == 'char':
            return pl.Utf8
        elif self.datatype == 'double':
            return pl.Float64
        elif self.datatype == 'float':
            return pl.Float32
        elif self.datatype == 'int':
            return pl.Int32
        elif self.datatype == 'long':
            return pl.Int64
        elif self.datatype == 'short':
            return pl.Int16
        elif self.datatype == 'boolean':
            return pl.Boolean
        else:
            return pl.Utf8

    def to_json(self) -> str:
        return json.dumps({key: value if key != 'unit' else repr(value) for key, value in vars(self).items()})

    @classmethod
    def from_json(cls, json_str: str):
        json_dict = json.loads(json_str)
        try:
            json_dict['unit'] = u.Unit(json_dict['unit'])
        except ValueError:
            json_dict['unit'] = None

        return cls(**json_dict)
    


@dataclass
class TableMetadata:
    access_url: str
    name: str
    title: str
    description: str
    fields: List[FieldMetadata]
    exception: Optional[Exception] = None

    @classmethod
    def from_cone_search_result(cls, resource: vo.registry.regtap.RegistryResource, result: vo.dal.scs.SCSResults):
        return cls(
            access_url=resource.access_url,
            name=resource.short_name,
            title=resource.res_title,
            description=resource.res_description,
            fields=[FieldMetadata.from_field(field) for field in result.fielddescs],
        )

    def to_json(self) -> str:
        return json.dumps({key: value if key != 'fields' else [f.to_json() for f in value] for key, value in vars(self).items() if not key.startswith('_')})

    def save(self, path: str):
        """ Save the metadata in a json file at the given path. """
        with open(path, 'w') as f:
            f.write(self.to_json())

    def empty_polars_frame(self) -> pl.DataFrame:
        """ Convert the metadata to an empty polars frame. 
        
        This can be used to create a polars DataFrame with the correct column
        names and types.

        More information about polars can be found here:
        https://pola-rs.github.io/polars-book/user-guide/dataframe.html
        """
        return pl.DataFrame(
            {
                field.name: pl.Series(name=field.name, dtype=field.to_polars_dtype())
                for field in self.fields
            }
        )

    def __bool__(self):
        return self.exception is None
        
    def check_schema(self, test_frame: pl.DataFrame) -> pl.DataFrame:
        """ Attempt to coerce the schema of the given DataFrame to match the metadata.
        
        If the schema of the DataFrame is not compatible with the metadata, an
        exception will be added to the metadata.
        """
        try:
            return self.empty_polars_frame().extend(test_frame)
        except Exception as e:
            if self.exception is None:
                self.exception = e
            else:
                self.exception = (self.exception, e)
        return test_frame

    @classmethod
    def from_json_string(cls, json_str: str) -> 'TableMetadata':
        json_dict = json.loads(json_str)
        json_dict['fields'] = [FieldMetadata.from_json(f) for f in json_dict['fields']]
        return cls(**json_dict)
    
    @classmethod
    def from_json_file(cls, path: str) -> 'TableMetadata':
        with open(path, 'r') as f:
            return cls.from_json_string(f.read())
        
    @classmethod
    def from_json(cls, json_or_path_to_json: str) -> 'TableMetadata':
        if json_or_path_to_json.endswith('.json'):
            return cls.from_json_file(json_or_path_to_json)
        else:
            return cls.from_json_string(json_or_path_to_json)
        
    @classmethod
    def load_failure(cls, e: Exception) -> 'TableMetadata':
        return cls(
            access_url='Failed to load',
            name='Failed to load',
            title='Failed to load',
            description='Failed to load',
            fields=[],
            exception=e,
        )
    
    

class Table:
    DEFAULT_METADATA_FILE_NAME = 'metadata.json'
    DEFAULT_DATA_FILE_NAME = 'data.parquet'

    def __init__(self, metadata: TableMetadata, data: pl.DataFrame):
        self.metadata = metadata
        self.data = data

    @classmethod
    def from_cone_search(cls, resource: vo.registry.regtap.RegistryResource, coord, radius):
        result = resource.search(coord, radius*u.deg)
        return cls(
            metadata=TableMetadata.from_cone_search_result(resource, result),
            data=pl.DataFrame(data=[dict(row) for row in result]),
        )
    
    @property
    def name(self):
        return self.metadata.name
    
    @property
    def description(self):
        return self.metadata.description
    
    @property
    def fields(self):
        return self.metadata.fields




    def save(self, folder_path: str):
        """ Save the data and metadata in a folder """
        os.makedirs(folder_path, exist_ok=True)
        self.metadata.save(os.path.join(folder_path, self.DEFAULT_METADATA_FILE_NAME))
        self.data.write_parquet(os.path.join(folder_path, self.DEFAULT_DATA_FILE_NAME))





    @classmethod
    def load(cls, folder_path: str) -> 'Table':
        """ Load the data and metadata from a folder """
        metadata = TableMetadata.from_json_file(os.path.join(folder_path, cls.DEFAULT_METADATA_FILE_NAME))
        
        if metadata:
            data = metadata.empty_polars_frame()
        try:
            raw_data = pl.read_parquet(os.path.join(folder_path, cls.DEFAULT_DATA_FILE_NAME))
            data = metadata.check_schema(raw_data)

        except FileNotFoundError as e:
            pass

        return cls(metadata, data)

    def __repr__(self):
        return f'Table(name={self.name},\ntitle={self.metadata.title} description={self.description})'


def tap_query(query: str, url: str='http://dc.zah.uni-heidelberg.de/__system__/tap/run/tap') -> Table:
    """ Perform a TAP query. """
    service = dal.TAPService(url)
    result = service.search(query)
    return Table(
        metadata=TableMetadata(
            access_url=url,
            name='TAP Query',
            title='TAP Query',
            description='TAP Query',
            fields=[FieldMetadata.from_field(field) for field in result.fielddescs],
        ),
        data=pl.DataFrame(data=[dict(row) for row in result]))


def cone_search(center: ap.coordinates.SkyCoord, radius: u.Quantity, waveband: str='optical', verbose: bool = False) -> List[Table]:
    """ Perform a cone search around the given center and radius. """
    # Get the list of resources
    resources = registry.search(
        pos=center,
        radius=radius,
        waveband=waveband,
        servicetype='conesearch'
    )

    # Perform the cone search on each resource
    tables = []
    with ThreadPoolExecutor(max_workers=10) as executor:
        futures = [executor.submit(Table.from_cone_search, resource, center, radius) for resource in resources]
        for future in as_completed(futures):
            try:
                tables.append(future.result())
            except Exception as e:
                if verbose:
                    print(e)
    return tables

def load_table(folder_path: str) -> Table:
    """ Load a table from a folder. """
    return Table.load(folder_path)

def load_tables_from(folder_path: str) -> List[Table]:
    """ Load a list of tables from a folder. """
    tables = []
    for folder in os.listdir(folder_path):
        tables.append(load_table(folder_path + '/' + folder))
    return tables

def yield_tables_from(directory: str) -> Iterator[Table]:
    """ Yield a list of tables from a folder. """
    for dir_entry in os.scandir(directory):
        if dir_entry.is_dir():
            if any([f.name == 'metadata.json' or f.name == 'data.parquet' for f in os.scandir(dir_entry.path)]):
                yield load_table(dir_entry.path)
            
            if any([f.is_dir() for f in os.scandir(dir_entry.path)]):
                for table in yield_tables_from(dir_entry.path):
                    yield table



def save_table(table: Table, folder_path: str):
    """ Save a table in a folder. """
    table.save(folder_path)

def save_tables(tables: List[Table], folder_path: str):
    """ Save a list of tables in a folder. """
    for table in tables:
        save_table(table, folder_path + '/' + table.name)


def search_tables(search_parameters: Dict[str, str], directory: str) -> Iterator[Table]:
    """ Search for tables in a directory that match the given search parameters.

    Search parameters are key-value pairs that are matched against various
    metadata fields of the tables, as well as the data itself.

    The following are the types of parameters that can be searched for:

        * `name`: The name of the table.
        
        * `title`: The title of the table.
        
        * `description`: The description of the table.
        
        * `field_name`: The name of a field in the table.
        
        * `field_description`: The description of a field in the table.

        * `field_ucd`: The UCD of a field in the table. The UCD is a standard
            that describes the meaning of a field.
        
        * `field_unit`: The unit of a field in the table.
        
        * `filter`: A filter to apply to the table data. The value assigned to
            this key should be a either a polars expression which evaluates to
            a boolean, or a function that takes the `Table` object and returns
            either a boolean or a `Table` object.

            If a `Table` object is returned that object will be added in the
            results, otherwise the original table will be added if the filter
            evaluates to `True`.

    The values supplied to the above parameters can be either a single value,
    or a list of values. If a list of values is supplied, then by default the
    table will be added to the results if any of the values match. 

    If any of the above parameters is suffixed with `_all`, then the table
    will only be added to the results if all of the values match.

    If any of the above parameters except `filter` are suffixed with `_regex`,
    then the value will be treated as a regular expression.

    If any of the above parameters except `filter` are suffixed with `_not`,
    then the value will be treated as a negative match.

    If any of the above parameters except `filter` are suffixed with `_like`,
    then the value will be treated as a SQL LIKE expression.

    Args:
        search_parameters: A dictionary of search parameters.
        directory: The directory to search in.
    """
    for table in yield_tables_from(directory):
        for search_key, search_values in search_parameters.items():
            if isinstance(search_values, str):
                search_values = [search_values]
            
            if not hasattr(search_values, '__iter__'):
                search_values = [search_values]
            
            
            for search_value in search_values:
                matched = _match_table(table, search_key, search_value)
                if matched:
                    if isinstance(matched, Table):
                        yield matched
                    else:
                        yield table
                        break

def _match_table(table: Table,
                 search_key: str,
                 search_value: Union[str, pl.Expr, Callable[[Table], Union[bool, Table]]]) -> Union[bool, Table]:
    """ Match a table against a search parameter. As described in the docstring
    of `search_tables`. """
    # print(search_key, search_value)
    search_key = search_key.casefold()
    if search_key == 'filter':
        if isinstance(search_value, pl.Expr):
            return len(table.data.filter(search_for)) > 0
        elif callable(search_value):
            return search_value(table)
        else:
            raise TypeError(f'Invalid type for filter parameter: {type(search_for)}')
    
    search_for = search_value.casefold()
    search_in = None

    search_key, _, suffix = search_key.rpartition('_')
    if not all([search_key, suffix]):
        search_key = search_key or suffix
        suffix = None
    try:
        if search_key == 'name':
            search_in = table.name
        elif search_key == 'title':
            search_in = table.metadata.title
        elif search_key == 'description':
            search_in = table.metadata.description
        elif search_key.startswith('field'):
            _, _, field_attr = search_key.rpartition('_')
            try:
                search_in = [getattr(field, field_attr) for field in table.fields]
            except AttributeError:
                raise ValueError(f'Invalid field attribute: {field_attr}')
        else:
            raise ValueError(f'Invalid search parameter: {search_key}')
    except AttributeError:
        raise ValueError(f'Invalid search parameter: {search_key}')

    if suffix == 'regex':
        return _match_table_regex(search_in, search_for)
    elif suffix == 'not':
        return _match_table_not(search_in, search_for)
    elif suffix == 'like':
        return _match_table_like(search_in, search_for)
    else:
        return _match_table_str(search_in, search_for)

def _match_table_regex(search_in: Union[str, List[str]], search_for: str) -> bool:
    """ Match a table against a search parameter that is a regular expression. """
    if isinstance(search_in, list):
        for value in search_in:
            if re.match(search_for, value):
                return True
        return False
    else:
        return re.match(search_for, search_in)
    
def _match_table_not(search_in: Union[str, List[str]], search_for: str) -> bool:
    """ Match a table against a search parameter that is a negative match. """
    if isinstance(search_in, list):
        search_for = search_for.casefold()
        
        for value in search_in:
            if _match_table(value, search_for):
                return False
        return True
    return not _match_table(search_in, search_for)

def _match_table_like(value_to_match: Union[str, List[str]], search_value: str) -> bool:
    """ Match a table against a search parameter that is a SQL LIKE expression. """
    if isinstance(value_to_match, list):
        for value in value_to_match:
            regex = fnmatch.translate(value, search_value)
            return _match_table_regex(value, regex)
    else:
        regex = fnmatch.translate(value, search_value)
        return _match_table_regex(value, regex)
    
def _match_table_str(value_to_match: Union[str, List[str]], search_value: str) -> bool:
    """ Match a table against a search parameter that is a string. """
    if isinstance(value_to_match, list):
        for value in value_to_match:
            if value == search_value:
                return True
        return False
    else:
        return value_to_match == search_value
    


