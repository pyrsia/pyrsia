FROM pyrsiaoss/codecoverage:1.0

COPY . /home/pyrsia/
WORKDIR /home/pyrsia
ENTRYPOINT ["cargo", "tarpaulin", "--workspace"]
