from qvd import read_qvd
import os
import pandas as pd

qvd = read_qvd([f'{os.path.dirname(__file__)}/test_files/test_qvd.qvd',f'{os.path.dirname(__file__)}/test_files/AAPL.qvd'],"1",True)
print(qvd)