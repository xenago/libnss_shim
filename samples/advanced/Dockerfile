# EL distro example
FROM docker.io/almalinux:9

# Install Python to run the custom resolver scripts
RUN yum install -y python3

# Install latest release of libnss_shim available for this architecture
RUN curl -s https://api.github.com/repos/xenago/libnss_shim/releases/latest | grep "browser_download_url.*.$(uname -m)" | cut -d : -f 2,3 | tr -d \" | tr -d " " | xargs -n 1 curl -sLO
RUN rpm -Ui libnss_shim*.rpm
RUN rm libnss_shim*.rpm

# Import config and scripts
COPY --chown=root:root custom_config.json /etc/libnss_shim/config.json
RUN chmod 644 /etc/libnss_shim/config.json
COPY --chown=root:root custom-group-resolver.py /etc/libnss_shim/group-resolver.py
RUN chmod 644 /etc/libnss_shim/group-resolver.py
COPY --chown=root:root custom-passwd-resolver.py /etc/libnss_shim/passwd-resolver.py
RUN chmod 644 /etc/libnss_shim/passwd-resolver.py
COPY --chown=root:root custom-shadow-resolver.py /etc/libnss_shim/shadow-resolver.py
RUN chmod 640 /etc/libnss_shim/shadow-resolver.py

# Run bash by default
ENTRYPOINT ["/bin/bash"]
