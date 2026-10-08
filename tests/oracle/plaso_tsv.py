"""plaso's esedb/file_history events (psort -o json_line) as sorted TSV.

One line per event: identifier, parent identifier, original filename,
file attributes, USN, the time as a FILETIME (empty when plaso has none),
and the time's description.

    python3 -I tests/oracle/plaso_tsv.py out.jsonl > tests/oracle/plaso.tsv
    gzip -9n tests/oracle/plaso.tsv
"""

import json
import sys


def text(value):
    """A field as TSV text: empty when absent."""
    return "" if value is None else str(value)


def main(path):
    lines = []
    with open(path, encoding="utf-8") as events:
        for line in events:
            event = json.loads(line)
            if event.get("data_type") != "windows:file_history:namespace":
                continue
            fields = [
                event.get("identifier"),
                event.get("parent_identifier"),
                event.get("original_filename"),
                event.get("file_attribute"),
                event.get("usn_number"),
                event["date_time"].get("timestamp"),
                event.get("timestamp_desc"),
            ]
            lines.append("\t".join(text(field) for field in fields))
    for line in sorted(lines):
        print(line)


if __name__ == "__main__":
    main(sys.argv[1])
