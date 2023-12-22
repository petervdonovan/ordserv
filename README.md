# ordserv

To get started, run:

```sh
git clone https://github.com/petervdonovan/ordserv.git
cd ordserv
git submodule update --init --recursive
./build.sh
export PATH=$(pwd)/lf-ordserv/build/install/lf-cli/bin:$PATH
./go.sh
```

This will build all dependencies and will start running the Lingua Franca C federated tests. Results will be persisted in the `scratch` directory.
