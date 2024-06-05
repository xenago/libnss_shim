import os
import sys

fn = sys.argv[1]

group_entries = {
    "test-shim-users": "test-shim-users:x:2000:test-shim-user-1,test-shim-user-2,test-shim-user-3",
    "test-shim-user-1": "test-shim-user-1:x:2001:",
    "test-shim-user-2": "test-shim-user-2:x:2002:",
    "test-shim-user-3": "test-shim-user-3:x:2003:"
}

if fn == "--all":
    for entry in group_entries.values():
        print(entry)
elif fn == "--name":
    provided_name = os.getenv("LIBNSS_SHIM_GROUP_NAME", default = None)
    if provided_name is not None and provided_name in group_entries:
        print(group_entries[provided_name])
elif fn == "--id":
    provided_gid = os.getenv("LIBNSS_SHIM_GROUP_ID", default = None)
    if provided_gid is not None:
        for group in group_entries.values():
            if group.split(":")[2] == provided_gid:
                print(group)
