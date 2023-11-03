# Project264

To get started, run:

```sh
git clone https://github.com/petervdonovan/cs264-final-project.git
cd cs264-final-project
git submodule update --init --recursive
./build.sh
export PATH=$(pwd)/lf-264/build/install/lf-cli/bin:$PATH
./go.sh
```

This will build all dependencies and will start running the Lingua Franca C federated tests. Results will be persisted in the `scratch` directory.
