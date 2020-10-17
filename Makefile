CARGO = cargo

install-diesel-cli:
	$(CARGO) install diesel_cli --features barrel-migrations,barrel/pg,postgres
