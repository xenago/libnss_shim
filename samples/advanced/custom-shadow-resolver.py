import os
import sys

fn = sys.argv[1]

# Each test user's password is the same as its username
entries = {
    "test-shim-user-1": "test-shim-user-1:$y$j9T$mpqMRQPh51zsMQlg6Koa5/$iYcT2urasxmk99rWCuahIEcNEQDGZcVN0876t80XUm2:19879:0:99999:7:::",
    "test-shim-user-2": "test-shim-user-2:$y$j9T$SEsXgfv/SUN3EZQJqLfIA/$mG9uKqlqDOqY2oYzuu1O89nmf1BiYs2//3rPof97vq9:19879:0:99999:7:::",
    "test-shim-user-3": "test-shim-user-3:$y$j9T$loMKkB7paRkhAPE7VUa9I.$7CoM0O7XZASdb4olZ8w3YkyjMw2TpoBjlUynOXDLOEB:19879:0:99999:7:::"
}

if fn == "--all":
    for entry in entries.values():
        print(entry)
elif fn == "--name":
    provided_name = os.getenv("LIBNSS_SHIM_SHADOW_NAME", default = None)
    if provided_name is not None and provided_name in entries:
        print(entries[provided_name])
