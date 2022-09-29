# KLV

KLV (Key-Length-Value) is a data encoding standard.

Gstreamerの`meta/x-klv`で使うことが目標である。
基本となるライブラリでは16ByteのヘッダとBERエンコードされたLengthは `KLVGlobal` で取り除き、
中のコンテンツは全て1Byteのキーで書かれていることを想定する。

## TODO

- [x] Try parsing USA Datalink Local Set.
- [x] Try Embedding `meta/x-klv` into mpeg2tsvideo
