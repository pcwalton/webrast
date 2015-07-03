#!/usr/bin/env node

var fs = require('fs');

const SIZE = 512;
const RADIUS = SIZE / 2;
const DISTANCE_SCALING_FACTOR = SIZE * Math.sqrt(2);
const DISTANCE_TO_COLOR_VALUE_FACTOR = 128.0;

const SIGMA = Math.sqrt(0.02);

function clamp(value, low, high) {
    if (value < low)
        return low;
    if (value > high)
        return high;
    return value;
}

var buffer = new Buffer(SIZE * SIZE);
for (var y = 0; y < SIZE; y++) {
    for (var x = 0; x < SIZE; x++) {
        var distanceToCenter = Math.sqrt((SIZE - y) * (SIZE - y) + (SIZE - x) * (SIZE - x));
        var distance = distanceToCenter - RADIUS;
        var b = Math.floor((1.0 - distance / DISTANCE_SCALING_FACTOR) *
                DISTANCE_TO_COLOR_VALUE_FACTOR);
        buffer[y * SIZE + x] = b;
    }
}

var twoSigmaSquared = 2.0 * SIGMA * SIGMA;
var a = 1.0 / (SIGMA * Math.sqrt(2.0 * Math.PI));
var blurBuffer = new Buffer(SIZE * SIZE);
for (var y = 0; y < SIZE; y++) {
    for (var x = 0; x < SIZE; x++) {
        // 0.0-0.5: outside arc; 0.5-1.0: inside arc
        var distance = buffer[y * SIZE + x] / DISTANCE_TO_COLOR_VALUE_FACTOR / 2.0;
        /*if (distance > 0.0) {
            blurBuffer[y * SIZE + x] = 255;
            continue;
        }*/

        var gaussianDistance = (a * Math.exp(-(distance * distance) / twoSigmaSquared));
        //gaussianDistance = clamp(gaussianDistance, 1.0, 3.0);
        //console.log(gaussianDistance);
        console.log(distance);
        //var gaussianDistance = distance;
        blurBuffer[y * SIZE + x] =
            Math.floor(gaussianDistance * DISTANCE_TO_COLOR_VALUE_FACTOR * 2.0);
    }
}

var fileBuffer = new Buffer(18 + SIZE * SIZE * 3);
for (var i = 0; i < 18; i++)
    fileBuffer[i] = 0;
fileBuffer[2] = 2;
fileBuffer[12] = SIZE & 0xff;
fileBuffer[13] = (SIZE >> 8) & 0xff; 
fileBuffer[14] = SIZE & 0xff;
fileBuffer[15] = (SIZE >> 8) & 0xff; 
fileBuffer[16] = 24;
for (var y = 0; y < SIZE; y++) {
    for (var x = 0; x < SIZE; x++) {
        var b = blurBuffer[(SIZE - y - 1) * SIZE + x];
        fileBuffer[18 + (y * SIZE + x) * 3 + 0] = b;
        fileBuffer[18 + (y * SIZE + x) * 3 + 1] = b;
        fileBuffer[18 + (y * SIZE + x) * 3 + 2] = b;
    }
}
fs.writeFileSync('arc-distance-field.tga', fileBuffer);

