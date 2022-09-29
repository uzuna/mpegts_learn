.PHONY: setup
setup: testdata/DayFlight.mpg
testdata/DayFlight.mpg:
	mkdir -p testdata
	wget https://samples.ffmpeg.org/MPEG2/mpegts-klv/Day%20Flight.mpg -O testdata/DayFlight.mpg