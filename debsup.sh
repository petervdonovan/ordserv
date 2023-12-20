cd reactor-c-ordserv/ && \
git add . && \
git commit --amend -m "REVERT ME" && \
cd ../lf-ordserv && \
git add . && \
git commit --amend -m "REVERT ME" && \
cd .. && \
./build.sh && ./go.sh
