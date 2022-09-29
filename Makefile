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

ci: .github/workflows/ci.yml
.github/workflows/ci.yml: cisupport/workflows/ci.yml
	cd cisupport && cargo run