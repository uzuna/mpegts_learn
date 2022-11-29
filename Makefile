.PHONY: setup
setup: testdata/DayFlight.mpg
testdata/DayFlight.mpg:
	mkdir -p testdata
	wget https://samples.ffmpeg.org/MPEG2/mpegts-klv/Day%20Flight.mpg -O testdata/DayFlight.mpg

.PHONY: testplay
testplay: setup
	cd gstapp && cargo run decode

.PHONY: demosave
demosave:
	cd gstapp && cargo run klv -s ../test.m2ts

.PHONY: demoplay
demoplay:
	cd gstapp && cargo run decode ../test.m2ts

.PHONY: generate_ci_settings
generate_ci_settings: .github/workflows/*.yml
.github/workflows/*.yml: cisupport/workflows/*.yml cisupport/src/main.rs
	cd cisupport && cargo run

.PHONY: fixfmt
fixfmt:
	cargo fmt --all
	# cargo fixはunstagedなファイルがあると動かないため
	git add -u
	cargo fix --allow-staged --all-targets --all-features

.PHONY: check-fmt
check-fmt:
	cargo fmt --all -- --check
	cargo clippy --all-targets --all-features -- -D warnings