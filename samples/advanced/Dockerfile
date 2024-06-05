# EL distro example
FROM docker.io/almalinux:9

# Install Python to run the `custom-resolver.py` script
RUN yum install -y python3

# Install latest release of libnss_shim available for this architecture
RUN curl -s https://api.github.com/repos/xenago/libnss_shim/releases/latest | grep "browser_download_url.*.$(uname -m)" | cut -d : -f 2,3 | tr -d \" | tr -d " " | xargs -n 1 curl -sLO
RUN rpm -Ui libnss_shim*.rpm
RUN rm libnss_shim*.rpm

# Import config and scripts
COPY custom_config.json /etc/libnss_shim/config.json
COPY custom-resolver.py /etc/libnss_shim/resolver.py

# Run bash by default
ENTRYPOINT ["/bin/bash"]
