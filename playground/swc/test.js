function cov_n2bsq7rmo() {
    var path = "/Users/travzhang/github.com/canyon-project/swc-plugin-istanbul/playground/babel/src/file.js";
    var hash = "108d7ce84274bdc3310736aef3f37b6182ba3adb";
    var global = new Function("return this")();
    var gcv = "__coverage__";
    var coverageData = {
        path: "/Users/travzhang/github.com/canyon-project/swc-plugin-istanbul/playground/babel/src/file.js",
        statementMap: {
            "0": {
                start: {
                    line: 2,
                    column: 4
                },
                end: {
                    line: 2,
                    column: 17
                }
            },
            "1": {
                start: {
                    line: 5,
                    column: 0
                },
                end: {
                    line: 5,
                    column: 22
                }
            }
        },
        fnMap: {
            "0": {
                name: "add",
                decl: {
                    start: {
                        line: 1,
                        column: 9
                    },
                    end: {
                        line: 1,
                        column: 12
                    }
                },
                loc: {
                    start: {
                        line: 1,
                        column: 18
                    },
                    end: {
                        line: 3,
                        column: 1
                    }
                },
                line: 1
            }
        },
        branchMap: {},
        s: {
            "0": 0,
            "1": 0
        },
        f: {
            "0": 0
        },
        b: {},
        _coverageSchema: "1a1c01bbd47fc00a2c39e90264f33305004495a9",
        hash: "108d7ce84274bdc3310736aef3f37b6182ba3adb"
    };
    var coverage = global[gcv] || (global[gcv] = {});
    if (!coverage[path] || coverage[path].hash !== hash) {
        coverage[path] = coverageData;
    }
    var actualCoverage = coverage[path];
    {
        // @ts-ignore
        cov_n2bsq7rmo = function () {
            return actualCoverage;
        };
    }
    return actualCoverage;
}
cov_n2bsq7rmo();
function add(a, b) {
    cov_n2bsq7rmo().f[0]++;
    cov_n2bsq7rmo().s[0]++;
    return a + b;
}
cov_n2bsq7rmo().s[1]++;
console.log(add(1, 2));