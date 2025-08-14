import os
import sys

fn = sys.argv[1]

entries = {
    "test-shim-user-1": "test-shim-user-1:x:2001:2001::/home/test-shim-user-1:/bin/bash",
    "test-shim-user-2": "test-shim-user-2:x:2002:2002::/home/test-shim-user-2:/bin/bash",
    "test-shim-user-3": "test-shim-user-3:x:2003:2003::/home/test-shim-user-3:/bin/bash"
}

if fn == "--all":
    for entry in entries.values():
        print(entry)
elif fn == "--name":
    provided_name = os.getenv("LIBNSS_SHIM_PASSWD_NAME", default = None)
    if provided_name is not None and provided_name in entries:
        print(entries[provided_name])
elif fn == "--id":
    provided_uid = os.getenv("LIBNSS_SHIM_PASSWD_ID", default = None)
    if provided_uid is not None:
        for entry in entries.values():
            if entry.split(":")[2] == provided_uid:
                print(entry)
