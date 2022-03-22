#!/usr/bin/env python3
import os
import subprocess
import sys


class VersionChange:
    def __init__(self, file_path, old_text, new_text_template, encoding="utf-8"):
        self.file_path = file_path
        self.old_text = old_text
        self.new_text_template = new_text_template
        self.encoding = encoding

    def perform(self, short_git_hash):
        with open(self.file_path, "r", encoding=self.encoding) as f:
            body = f.read()

        new_text = self.new_text_template.format(
            SHORT_GIT_HASH=short_git_hash,
        )
        body = body.replace(self.old_text, new_text)

        with open(self.file_path, "w", encoding=self.encoding) as f:
            f.write(body)


CHANGES = [
    VersionChange(
        file_path=os.path.join("templates", "base.html"),
        old_text='<meta name="generator" content="wordle-archive" />',
        new_text_template='<meta name="generator" content="wordle-archive {SHORT_GIT_HASH}" />',
    ),
]


def get_output(args):
    process = subprocess.Popen(
        args,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    (cmd_stdout, cmd_stderr) = process.communicate()
    if process.wait() != 0:
        sys.stdout.buffer.write(cmd_stdout)
        sys.stderr.buffer.write(cmd_stderr)
        sys.exit(process.returncode)

    return cmd_stdout.decode().strip()


def main():
    short_git_hash = get_output(["git", "show", "--pretty=tformat:%h", "--no-patch", "HEAD"])

    for change in CHANGES:
        change.perform(short_git_hash)


if __name__ == '__main__':
    main()
