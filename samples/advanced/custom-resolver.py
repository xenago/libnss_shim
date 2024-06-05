import os
import sys

# Expected syntax example: `resolver.py --group --all`
# The database is `--group` and the function is `--all`
db = sys.argv[1]
fn = sys.argv[2]

group_entries = {
    "test-shim-users": "test-shim-users:x:2000:test-shim-user-1,test-shim-user-2,test-shim-user-3",
    "test-shim-user-1": "test-shim-user-1:x:2001:",
    "test-shim-user-2": "test-shim-user-2:x:2002:",
    "test-shim-user-3": "test-shim-user-3:x:2003:"
}

passwd_entries = {
    "test-shim-user-1": "test-shim-user-1:x:2001:2001::/home/test-shim-user-1:/bin/bash",
    "test-shim-user-2": "test-shim-user-2:x:2002:2002::/home/test-shim-user-1:/bin/bash",
    "test-shim-user-3": "test-shim-user-3:x:2003:2003::/home/test-shim-user-1:/bin/bash"
}

# Each test user's password is the same as its username
shadow_entries = {
    "test-shim-user-1": "test-shim-user-1:$y$j9T$mpqMRQPh51zsMQlg6Koa5/$iYcT2urasxmk99rWCuahIEcNEQDGZcVN0876t80XUm2:19879:0:99999:7:::",
    "test-shim-user-2": "test-shim-user-2:$y$j9T$SEsXgfv/SUN3EZQJqLfIA/$mG9uKqlqDOqY2oYzuu1O89nmf1BiYs2//3rPof97vq9:19879:0:99999:7:::",
    "test-shim-user-3": "test-shim-user-3:$y$j9T$loMKkB7paRkhAPE7VUa9I.$7CoM0O7XZASdb4olZ8w3YkyjMw2TpoBjlUynOXDLOEB:19879:0:99999:7:::"
}

if db == "--group":
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
elif db == "--passwd":
    if fn == "--all":
        for entry in passwd_entries.values():
            print(entry)
    elif fn == "--name":
        provided_name = os.getenv("LIBNSS_SHIM_PASSWD_NAME", default = None)
        if provided_name is not None and provided_name in passwd_entries:
            print(passwd_entries[provided_name])
    elif fn == "--id":
        provided_uid = os.getenv("LIBNSS_SHIM_PASSWD_ID", default = None)
        if provided_uid is not None:
            for user_entry in passwd_entries.values():
                if user_entry.split(":")[2] == provided_uid:
                    print(user_entry)
elif db == "--shadow":
    if fn == "--all":
        for entry in shadow_entries.values():
            print(entry)
    elif fn == "--name":
        provided_name = os.getenv("LIBNSS_SHIM_SHADOW_NAME", default = None)
        if provided_name is not None and provided_name in shadow_entries:
            print(shadow_entries[provided_name])
