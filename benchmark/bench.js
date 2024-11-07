"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
const benny_1 = __importDefault(require("benny"));
const index_1 = require("../index");
function add(a) {
    return a + 100;
}
async function run() {
    await benny_1.default.suite('Add 100', benny_1.default.add('Native a + 100', () => {
        (0, index_1.plus100)(10);
    }), benny_1.default.add('JavaScript a + 100', () => {
        add(10);
    }), benny_1.default.cycle(), benny_1.default.complete());
}
run().catch((e) => {
    console.error(e);
});
