(.id |= sub("^prefork-rs-"; "pfd-")) |
(.dependencies[]?.issue_id |= sub("^prefork-rs-"; "pfd-")) |
(.dependencies[]?.depends_on_id |= sub("^prefork-rs-"; "pfd-"))
