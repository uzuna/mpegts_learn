# CI Support util

Github Actionsを使ってテストを実行している
構文にAnchorが使えない制約があるため、ここで書いてそれを生成するフローにしている。


## Why

### test.yml

CI test向けの設定
workspace全体をtestしている。
差分テストを検討しているが、安定した差分取得の方法が検討できていないので全体をテストしている。
`Swatinem/rust-cache@v2`によりビルドキャッシュが利用できているので差分テストにするモチベーションはまだない


### bench.yml

パフォーマンスが変化しないことを記録に残したいために作成している。
CI環境ではCPUも変わると思われるため、正確な記録ではないが変化は検知が出来るため大まかな性能評価には使えるかと考えている。
書き方は[GitHub Docs Events that trigger workflows](https://docs.github.com/en/actions/using-workflows/events-that-trigger-workflows#pull_request)を参照した。
master向けのPRが作成、更新されたときだけ行う。

ベンチマーク結果を何処かに書き出したいとは思っている。
