
const path = require('path');

module.exports = {
    mode: "development",
    devtool: 'source-map',
    entry: './src/sdk.ts',
    output: {
        filename: 'sdk.js',
        path: path.resolve(__dirname, 'build'),
        library: "MyLibrary",
        libraryTarget: 'umd',
        clean: true
    },
    module: {
        rules: [
            {
                test: /\.(js)x?$/,
                exclude: /node_modules/,
                use: "babel-loader",
            },
            {
                test: /\.(ts)x?$/,
                exclude: /node_modules|\.d\.ts$/, // this line as well
                use: {
                    loader: "ts-loader",
                    options: {
                        compilerOptions: {
                            noEmit: false, // this option will solve the issue
                        },
                    },
                },
            },
        ]
    },
    resolve: {
        extensions: ['.ts', '.js', '.json']
    },
};
