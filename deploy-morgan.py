#!/usr/bin/env python3
"""
Will deploy nightly Bitconch chain, codename morgan 
"""
import logging
import stat
import shutil
import os, re, argparse, sys,crypt
import getpass
import click
from subprocess import Popen, check_call, PIPE, check_output, CalledProcessError
from shutil import copy2, copytree, rmtree
from colorama import init
init()
from colorama import Fore, Back, Style

def rmtree_onerror(self, func, file_path, exc_info):
    """
    Error handler for ``shutil.rmtree``.
    If the error is due to an access error (read only file)
    it attempts to add write permission and then retries.
    If the error is for another reason it re-raises the error.
    Usage : ``shutil.rmtree(path, onerror=onerror)`` 
    """
    logging.warning(str(exc_info))
    logging.warning("rmtree error,check the file exists or try to chmod the file,then retry rmtree action.")
    os.chmod(file_path, stat.S_IWRITE) #chmod to writeable
    if os.path.isdir(file_path):
        #file exists
       func(file_path)
       else:
        #handle whatever
        raise


def execute_shell(command, silent=False, cwd=None, shell=True, env=None):
    """
    Execute a system command 
    """
    if env is not None:
        env = dict(**os.environ, **env)

    if silent:
        p = Popen(
            command, shell=shell, stdout=PIPE, stderr=PIPE, cwd=cwd, env=env)
        stdout, _ = p.communicate()

        return stdout
    else:
        check_call(command, shell=shell, cwd=cwd, env=env)

def prnt_warn(in_text):
    """
    Print a warning message
    """
    print(Fore.YELLOW + "[!]"+in_text)
    print(Style.RESET_ALL)

def prnt_run(in_text):
    """
    Print a processing message
    """
    print(Fore.WHITE + "[~]"+in_text)
    print(Style.RESET_ALL)

def prnt_error(in_text):
    """
    Print an error message
    """
    print(Fore.RED + "[~]"+in_text)
    print(Style.RESET_ALL)

def update_submodules():
    """
    Pull the latest submodule code from upstream
    """
    prnt_warn('This repo uses submodules to manage the codes')
    prnt_run("Use git to update the submodules")
    # Ensure the submodule is initialized
    execute_shell("git submodule update --init --recursive", silent=False)
