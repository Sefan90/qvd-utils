from qvd import read_qvd
import os
import pandas as pd

qvd = read_qvd(f'{os.path.dirname(__file__)}/test_files/test_qvd.qvd',"1")
print(qvd)