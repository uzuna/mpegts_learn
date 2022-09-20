
## MPEG2-TS

format

### Header

TransportStreamHeader: Minimum 4Byte

32bit


PID = Packet Identifier, describing the payload data.

#### PID

|Decimal|Hex|Description|
|:--|:--|:--|
|0|0x0000|Program association table (PAT) contains a directory listing of all program map tables|
|1|0x0001|Conditional access tableCAT)|



#### PAT

TSで使用可能な全てのプログラムの一覧。
PATがない場合はネットワーク情報テーブルNITがデフォルト値 `0x0010` を使う

- 2byte: program number
- 3bit: reserved
- 13bit: PMTID



## gstreamer recording mpeg


1. Audioは `audio/mpeg`の`streamtype adts`でなければならないため`voaacenc`を使う
1. Audioのサンプリングが早いので`queue ! audiorate`を挟むことでH264のエンコード時間とフレーム差を吸収してタイムスタンプ合わせさせる

gst-launch-1.0 v4l2src device="/dev/video0" ! 'video/x-raw, width=640, height=480, framerate=30/1' ! videoconvert ! x264enc key-int-max=30 ! h264parse ! queue ! m. autoaudiosrc ! audioconvert ! queue ! audiorate ! voaacenc ! queue ! m. mpegtsmux name=m ! multifilesink location=test%d.m2ts index=0 next-file=4 max-file-size=2097152
