cd reactor-c-264/ && \
git add . && \
git commit --amend -m "REVERT ME" && \
cd ../lf-264 && \
git add . && \
git commit --amend -m "REVERT ME" && \
cd .. && \
./build.sh && ./go.sh
