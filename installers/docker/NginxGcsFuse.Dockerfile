FROM ubuntu:focal

RUN apt-get update; \
    apt-get install -y curl gnupg2; \
    echo "deb http://packages.cloud.google.com/apt gcsfuse-focal main" | tee /etc/apt/sources.list.d/gcsfuse.list; \
    curl https://packages.cloud.google.com/apt/doc/apt-key.gpg | apt-key add -; \
    apt-get update; \
    apt-get install -y gcsfuse nginx;
    
RUN rm -rf /var/www/html; \
    mkdir -p /var/www/html; \
    sed -i "s/#user_allow_other/user_allow_other/" /etc/fuse.conf; \
    echo "debrepo /var/www/html gcsfuse rw,_netdev,allow_other,implicit_dirs,file_mode=777,dir_mode=777" >> /etc/fstab

CMD ["nginx", "-g", "daemon off;"]