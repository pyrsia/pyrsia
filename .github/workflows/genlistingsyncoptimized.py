#!/usr/bin/env python3
#
# Generate files and directories in the sync directories instead of rsyncing
# the entire content from cloud bucket everytime. genlisting.py script only needs
# the directory and file to be placed in the same hierarchical fashion to generate
# listing in index.html
import sys
import os

def cleanListing(filenames: str) -> list[str]:
    """Expect to receive listing of a cloud bucket from shell output and convert it to a python list"""
    filenames = filenames.splitlines()
    return list(filter(lambda filename: (filename != ""), filenames))

def getDirListAndFileList(filenames: list[str], str_to_drop: str) -> tuple[list[str], list[str]]:
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
        filename = filename[len(str_to_drop):]
        if filename.endswith(":"):
            dir_list.append(filename[0:-1])
        else:
            file_list.append(filename)
    return dir_list, file_list

def createDir(dir_list: list[str], syncdir: str) -> None:
    """
    Creates directories if doesn't exist from a list while prepending the value from syncdir
    """
    for dir in dir_list:
        if not os.path.exists(syncdir+"/"+dir):
            os.makedirs(syncdir+"/"+dir, exist_ok=True)

# Create files
def createFile(file_list: list[str], syncdir: str) -> None:
    """Creates files if doesn't exist from a list while prepending the value from syncdir"""
    for file in file_list:
        if not os.path.exists(syncdir+"/"+file):
            f = open(syncdir+"/"+file, 'w+')
            f.close()

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
