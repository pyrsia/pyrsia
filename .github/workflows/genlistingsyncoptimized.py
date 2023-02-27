#!/usr/bin/env python3
#
# Generate files and directories in the sync directories instead of rsyncing
# the entire content from cloud bucket everytime. genlisting.py script only needs
# the directory and file to be placed in the same hierarchical fashion to generate
# listing in index.html
# The program is expected to work with the output of "gsutil ls -lr gs://<bucket_name>"
import sys
import os
import re
import datetime
import calendar
from typing import List, Tuple

class Listing:
    def __init__(self, size: int, timestamp: str, name: str, is_dir: bool):
        self.size = size
        self.timestamp = timestamp
        self.name = name
        self.is_dir = is_dir

    @staticmethod
    def from_listing_text(listing_text: str, drop_name_prefix: str):
        datalist = re.split('\s+', listing_text.strip())
        if len(datalist) > 1:
            size = datalist[0]
            timestamp = datalist[1]
            name = datalist[2][len(drop_name_prefix):]
            is_dir = False
        else:
            size = 0
            timestamp = ""
            name = datalist[0][len(drop_name_prefix):][0:-1]
            is_dir = True
        return Listing(size, timestamp, name, is_dir)
    def __str__(self):
        return "size: "+ self.size + ", timestamp:" + self.timestamp + ", name:" + self.name + ", is_dir:" + self.is_dir

def cleanListing(filenames: str) -> List[str]:
    """Expect to receive listing of a cloud bucket from shell output and convert it to a python list"""
    filenames = filenames.splitlines()
    return list(filter(lambda filename: (filename != ""), filenames))

def getDirListAndFileList(filenames: List[str], str_to_drop: str) -> Tuple[List[Listing], List[Listing]]:
    """
    Expect to receive listing of a cloud bucket from shell output and convert it to a python list
    Attributes
    ----------
    filenames: List of strings
    str_to_drop: string
    """
    dir_list = []
    file_list = []
    for filename in filenames:
        listing = Listing.from_listing_text(filename, str_to_drop)
        if listing.is_dir:
            dir_list.append(listing)
        else:
            file_list.append(listing)
    return dir_list, file_list

def createDir(dir_list: List[Listing], syncdir: str) -> None:
    """
    Creates directories if doesn't exist from a list while prepending the value from syncdir
    """
    for dir in dir_list:
        dirpath = syncdir+"/"+dir.name
        if not os.path.exists(dirpath):
            os.makedirs(dirpath, exist_ok=True)

def createFile(file_list: List[Listing], syncdir: str) -> None:
    """Creates files if doesn't exist from a list while prepending the value from syncdir"""
    for file in file_list:
        filepath = syncdir+"/"+file.name
        if not os.path.exists(filepath):
            with open(filepath, 'w+') as f:
                f.write("file_size===="+file.size)
            file_timestamp = epoch_time_from_dateime_str(file.timestamp)
            os.utime(filepath, (file_timestamp, file_timestamp))

def epoch_time_from_dateime_str(date_time_str: str) -> int:
    date_time = datetime.datetime.strptime(date_time_str, '%Y-%m-%dT%H:%M:%SZ')
    return calendar.timegm(date_time.utctimetuple())

def main(argv):
    if len(argv) != 3:
        print("Program needs 3 arguments. <listing output> <begin phrase to drop> <syncdir>")
        sys.exit(1)
    listing = argv[0]
    init_phrase_to_drop = argv[1]
    syncdir = argv[2]
    filenames = cleanListing(listing)
    (dir_list, file_list) = getDirListAndFileList(filenames, init_phrase_to_drop)
    createDir(dir_list, syncdir)
    createFile(file_list, syncdir)

if __name__ == "__main__":
    main(sys.argv[1:])
