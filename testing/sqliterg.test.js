import { execa } from 'execa';
import { describe, test, afterEach } from '@jest/globals';
import * as fs from 'fs';

const command = "../target/debug/sqliterg";
let commandHandler;

const timeout = 10000;

async function curl(url = "http://localhost:12321/test/exec", request, http_auth) {
    const headers = {
        'Content-Type': 'application/json',
    };
    if (http_auth)
        headers['Authorization'] = `Basic ${btoa(`${http_auth.user}:${http_auth.password}`)}`
    const requestOptions = {
        method: 'POST',
        headers,
        body: JSON.stringify(request),
    };

    try {
        const response = await fetch(url, requestOptions);
        let data = await response.text();
        // console.log(data);
        data = JSON.parse(data);

        return {
            success: response.status == 200,
            statusCode: response.status,
            result: data,
        };
    } catch (error) {
        console.error(error.message);
        return {
            success: false,
            result: error.message,
        };
    }
}

async function pause() {
    return new Promise((r) => setTimeout(r, 1000));
}

afterEach(async () => {
    if (commandHandler) {
        commandHandler.kill();
    }
    try {
        await execa("pkill", ["sqliterg"]);
    } catch (e) { };
    await execa("bash", ["-c", "rm -f test.db*"]);
});

describe('Server Status', () => {
    test('Server doesn\'t start with no args', async () => {
        try {
            commandHandler = await execa(command, [], { detached: true });
            expect(true).toBe(false);
        } catch (error) {
            expect(true).toBe(true);
        }
    }, timeout);
    test('Server starts successfully', async () => {
        try {
            commandHandler = execa(command, ["--serve-dir", "."], { detached: true });
            expect(true).toBe(true);
        } catch (error) {
            console.error(error);
            expect(true).toBe(false);
        }
    }, timeout);
    test('Server starts successfully with mem db', async () => {
        try {
            commandHandler = execa(command, ["--mem-db", "test"], { detached: true });
            await pause();
            expect((await curl(undefined, { transaction: [{ query: "SELECT 1" }] })).success).toBe(true);
        } catch (error) {
            console.error(error);
            expect(true).toBe(false);
        }
    }, timeout);
    test('Server starts successfully with file db', async () => {
        try {
            commandHandler = execa(command, ["--db", "test.db"], { detached: true });
            await pause();
            expect(fs.existsSync("test.db")).toBe(true);
            expect((await curl(undefined, { transaction: [{ query: "SELECT 1" }] })).success).toBe(true);
        } catch (error) {
            console.error(error);
            expect(true).toBe(false);
        }
    }, timeout);
});
