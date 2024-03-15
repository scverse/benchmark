# Write the benchmarking functions here.
# See "Writing benchmarks" in the asv docs for more information.

import time


class TimeSuite:
    """
    An example benchmark that times the performance of various kinds
    of iterating over dictionaries in Python.
    """
    def setup(self):
        self.d = {}
        for x in range(500):
            self.d[x] = None

    def time_keys(self):
        for key in self.d.keys():
            pass

    def time_values(self):
        for value in self.d.values():
            pass

    def time_range(self):
        d = self.d
        time.sleep(1)
        for key in range(500):
            d[key]


class MemSuite:
    def mem_list(self):
        return [0] * 256
