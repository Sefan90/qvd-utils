from .qvd import read_qvd
import pandas as pd


def read(file_name,find_string):
    data = read_qvd(file_name,find_string)
    df = pd.DataFrame.from_dict(data)
    return df


def read_to_dict(file_name,find_string):
    data = read_qvd(file_name,find_string)
    return data
