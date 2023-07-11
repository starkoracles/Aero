const path = require('path');
const HtmlWebpackPlugin = require('html-webpack-plugin');

module.exports = {
    mode: "development",
    devtool: 'cheap-module-source-map',
    entry: './src/demo/index.ts',
    output: {
        filename: 'index.js'
    },
    optimization: {
        minimize: false,
    },
    module: {
        rules: [
            {
                test: /\.(m|j|t)s$/,
                exclude: /(node_modules|bower_components)/,
                use: {
                    loader: 'babel-loader'
                }
            },
            {
                test: /\.(ts)x?$/,
                exclude: /node_modules|\.d\.ts$/, // this line as well
                use: {
                    loader: "ts-loader"
                },
            },
            {
                test: /src\/hashing_worker\.ts$/,
                use: [{
                    loader: 'worker-loader',
                    options: {
                        filename: 'hashing_worker.js',
                    }
                }, { loader: 'ts-loader', }],
            },
            {
                test: /src\/proving_worker\.ts$/,
                use: [{
                    loader: 'worker-loader',
                    options: {
                        filename: 'proving_worker.js',
                    }
                }, { loader: 'ts-loader', }],
            },
            {
                test: /src\/constraints_worker\.ts$/,
                use: [{
                    loader: 'worker-loader',
                    options: {
                        filename: 'constraints_worker.js',
                    }
                }, { loader: 'ts-loader', }],
            },
        ]
    },
    plugins: [
        new HtmlWebpackPlugin(),
    ],
    resolve: {
        extensions: ['.ts', '.js', '.json']
    },
    experiments: {
        topLevelAwait: true
    }
};