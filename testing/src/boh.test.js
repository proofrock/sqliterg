import { execa } from 'execa';
import { describe, test, afterEach } from '@jest/globals';

let serverProcess;

afterEach(() => {
    if (serverProcess && !serverProcess.killed) {
        serverProcess.kill();
    }
});

describe('Server Status', () => {
    test('Server starts successfully', async () => {
        try {
            serverProcess = execa('../target/release/sqliterg --serve-dir .', [], { detached: true });
            await serverProcess;
        } catch (error) {
            console.error(error);
        }

        expect(serverProcess.killed).toBe(false);
    }, 1000);
});
