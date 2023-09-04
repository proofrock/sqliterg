// Copyright (c) 2023-, Germano Rizzo <oss /AT/ germanorizzo /DOT/ it>
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

package main

import (
	"bytes"
	"encoding/json"
	"io"
	"net/http"
	"os"
	"os/exec"
	"sync"
	"testing"
	"time"

	"github.com/stretchr/testify/require"
	"gopkg.in/yaml.v3"
)

const COMMAND = "../target/debug/sqliterg"

func TestMain(m *testing.M) {
	println("Go...")
	exitCode := m.Run()
	println("...finished")
	os.Exit(exitCode)
}

var cmd *exec.Cmd

func setupTest(t *testing.T, cfg *db, argv ...string) func(bool) {
	if cfg != nil {
		data, err := yaml.Marshal(cfg)
		require.NoError(t, err)

		require.NoError(t, os.WriteFile("env/test.yaml", data, 0600))
	}

	cmd = exec.Command(COMMAND, argv...)
	require.NoError(t, cmd.Start())

	time.Sleep(333 * time.Millisecond)

	return func(cleanFiles bool) {
		cmd.Process.Kill()
		if cleanFiles {
			os.Remove("env/test.db")
			os.Remove("env/test.db-shm")
			os.Remove("env/test.db-wal")
			os.Remove("env/test.yaml")
		}
	}
}

func TestErrorNoArgs(t *testing.T) {
	cmd := exec.Command(COMMAND, "--db", "env/test.db", "--mem-db", "test")
	defer os.Remove("env/test.db")
	defer os.Remove("env/test.db-shm")
	defer os.Remove("env/test.db-wal")
	err := cmd.Run()
	require.Error(t, err)
}

func TestErrorSameArgs(t *testing.T) {
	cmd := exec.Command(COMMAND)
	err := cmd.Run()
	require.Error(t, err)
}

func mkRaw(mapp map[string]interface{}) map[string]json.RawMessage {
	ret := make(map[string]json.RawMessage)
	for k, v := range mapp {
		bytes, _ := json.Marshal(v)
		ret[k] = bytes
	}
	return ret
}

func call(t *testing.T, url string, req request) (int, string, response) {
	reqbytes, err := json.Marshal(req)
	require.NoError(t, err)
	post, err := http.NewRequest("POST", url, bytes.NewBuffer(reqbytes))
	require.NoError(t, err)
	post.Header.Add("Content-Type", "application/json")

	resp, err := http.DefaultClient.Do(post)
	require.NoError(t, err)

	bs, err := io.ReadAll(resp.Body)
	require.NoError(t, err)
	ret := string(bs)
	var obj response
	json.Unmarshal(bs, &obj)

	return resp.StatusCode, ret, obj
}

func TestFileServer(t *testing.T) {
	defer setupTest(t, nil, "--serve-dir", ".")(true)

	resp, err := http.Get("http://localhost:12321/env/test.1")
	require.NoError(t, err)

	require.Equal(t, http.StatusOK, resp.StatusCode)
	bs, err := io.ReadAll(resp.Body)
	require.NoError(t, err)
	require.Equal(t, "1", string(bs))
}

func TestMemDbEmpty(t *testing.T) {
	defer setupTest(t, nil, "--mem-db", "test")(true)

	code, _, obj := call(t, "http://localhost:12321/test/exec", request{Transaction: []requestItem{{Query: "SELECT 1"}}})

	require.Equal(t, http.StatusOK, code)
	require.Equal(t, 1, len(obj.Results))
	require.True(t, obj.Results[0].Success)
	require.Equal(t, 1, len(obj.Results[0].ResultSet))
	require.Equal(t, 1, len(obj.Results[0].ResultSet[0]))
	require.Equal(t, 1, int(obj.Results[0].ResultSet[0]["1"].(float64)))
}

func TestMemDbEmptyAnotherPort(t *testing.T) {
	defer setupTest(t, nil, "--mem-db", "test", "--port", "32123")(true)

	code, _, _ := call(t, "http://localhost:32123/test/exec", request{Transaction: []requestItem{{Query: "SELECT 1"}}})

	require.Equal(t, http.StatusOK, code)
}

func TestStatementQueryMismatch(t *testing.T) {
	defer setupTest(t, nil, "--mem-db", "test")(true)

	code, _, _ := call(t, "http://localhost:12321/test/exec", request{Transaction: []requestItem{{Statement: "SELECT 1"}}})

	require.Equal(t, http.StatusBadRequest, code)
}

func TestFileDbEmpty(t *testing.T) {
	defer setupTest(t, nil, "--db", "env/test.db")(true)

	require.FileExists(t, "env/test.db")

	code, _, obj := call(t, "http://localhost:12321/test/exec", request{Transaction: []requestItem{{Query: "SELECT 1"}}})

	require.Equal(t, http.StatusOK, code)
	require.Equal(t, 1, len(obj.Results))
	require.True(t, obj.Results[0].Success)
	require.Equal(t, 1, len(obj.Results[0].ResultSet))
	require.Equal(t, 1, len(obj.Results[0].ResultSet[0]))
	require.Equal(t, 1, int(obj.Results[0].ResultSet[0]["1"].(float64)))
}

func TestAll3(t *testing.T) {
	defer setupTest(t, nil, "--db", "env/test.db", "--mem-db", "test2", "--serve-dir", ".")(true)

	require.FileExists(t, "env/test.db")

	code, _, obj := call(t, "http://localhost:12321/test/exec", request{Transaction: []requestItem{{Query: "SELECT 1"}}})

	require.Equal(t, http.StatusOK, code)
	require.Equal(t, 1, len(obj.Results))
	require.True(t, obj.Results[0].Success)
	require.Equal(t, 1, len(obj.Results[0].ResultSet))
	require.Equal(t, 1, len(obj.Results[0].ResultSet[0]))
	require.Equal(t, 1, int(obj.Results[0].ResultSet[0]["1"].(float64)))

	code, _, obj = call(t, "http://localhost:12321/test/exec", request{Transaction: []requestItem{{Query: "SELECT 1"}}})

	require.Equal(t, http.StatusOK, code)
	require.Equal(t, 1, len(obj.Results))
	require.True(t, obj.Results[0].Success)
	require.Equal(t, 1, len(obj.Results[0].ResultSet))
	require.Equal(t, 1, len(obj.Results[0].ResultSet[0]))
	require.Equal(t, 1, int(obj.Results[0].ResultSet[0]["1"].(float64)))
}

// The following tests are adapted from ws4sqlite

func TestCreate(t *testing.T) {
	defer setupTest(t, nil, "--db", "env/test.db")(true)
	req := request{
		Transaction: []requestItem{
			{
				Statement: "CREATE TABLE T1 (ID INT PRIMARY KEY, VAL TEXT NOT NULL)",
			},
		},
	}

	code, _, res := call(t, "http://localhost:12321/test/exec", req)

	require.Equal(t, http.StatusOK, code)
	require.True(t, res.Results[0].Success)
}

func TestFail(t *testing.T) {
	defer setupTest(t, nil, "--db", "env/test.db")(true)
	req := request{
		Transaction: []requestItem{
			{
				Statement: "CREATE TABLE T1 (ID INT PRIMARY KEY, VAL TEXT NOT NULL)",
			},
			{
				Statement: "CREATE TABLE T1 (ID INT PRIMARY KEY, VAL TEXT NOT NULL)",
			},
		},
	}

	code, _, _ := call(t, "http://localhost:12321/test/exec", req)

	require.Equal(t, http.StatusBadRequest, code)
}

func TestTx(t *testing.T) {
	defer setupTest(t, nil, "--db", "env/test.db")(true)
	req := request{
		Transaction: []requestItem{
			{
				Statement: "CREATE TABLE T1 (ID INT PRIMARY KEY, VAL TEXT NOT NULL)",
			},
			{
				Statement: "INSERT INTO T1 (ID, VAL) VALUES (1, 'ONE')",
			},
			{
				Statement: "INSERT INTO T1 (ID, VAL) VALUES (1, 'TWO')",
				NoFail:    true,
			},
			{
				Query: "SELECT * FROM T1 WHERE ID = 1",
			},
			{
				Statement: "INSERT INTO T1 (ID, VAL) VALUES (:ID, :VAL)",
				Values: mkRaw(map[string]interface{}{
					"ID":  2,
					"VAL": "TWO",
				}),
			},
			{
				Statement: "INSERT INTO T1 (ID, VAL) VALUES (:ID, :VAL)",
				ValuesBatch: []map[string]json.RawMessage{
					mkRaw(map[string]interface{}{
						"ID":  3,
						"VAL": "THREE",
					}),
					mkRaw(map[string]interface{}{
						"ID":  4,
						"VAL": "FOUR",
					})},
			},
			{
				Query: "SELECT * FROM T1 WHERE ID > :ID",
				Values: mkRaw(map[string]interface{}{
					"ID": 0,
				}),
			},
		},
	}

	code, _, res := call(t, "http://localhost:12321/test/exec", req)

	require.Equal(t, http.StatusOK, code)

	require.True(t, res.Results[1].Success)
	require.False(t, res.Results[2].Success)
	require.True(t, res.Results[3].Success)
	require.True(t, res.Results[4].Success)
	require.True(t, res.Results[5].Success)
	require.True(t, res.Results[6].Success)

	require.Equal(t, 1, *res.Results[1].RowsUpdated)
	require.Equal(t, "ONE", res.Results[3].ResultSet[0]["VAL"])
	require.Equal(t, 1, *res.Results[4].RowsUpdated)
	require.Equal(t, 2, len(res.Results[5].RowsUpdatedBatch))
	require.Equal(t, 1, res.Results[5].RowsUpdatedBatch[0])
	require.Equal(t, 4, len(res.Results[6].ResultSet))
}

func TestTxRollback(t *testing.T) {
	defer setupTest(t, nil, "--db", "env/test.db")(true)
	req := request{
		Transaction: []requestItem{
			{
				Statement: "CREATE TABLE T1 (ID INT PRIMARY KEY, VAL TEXT NOT NULL)",
			},
		},
	}

	call(t, "http://localhost:12321/test/exec", req)

	req = request{
		Transaction: []requestItem{
			{
				Statement: "INSERT INTO T1 (ID, VAL) VALUES (1, 'ONE')",
			},
			{
				Statement: "INSERT INTO T1 (ID, VAL) VALUES (1, 'ONE')",
			},
		},
	}

	code, _, _ := call(t, "http://localhost:12321/test/exec", req)

	require.Equal(t, http.StatusBadRequest, code)

	req = request{
		Transaction: []requestItem{
			{
				Query: "SELECT * FROM T1",
			},
		},
	}

	code, _, res := call(t, "http://localhost:12321/test/exec", req)

	require.Equal(t, http.StatusOK, code)

	require.True(t, res.Results[0].Success)
	require.Equal(t, 0, len(res.Results[0].ResultSet))
}

func TestStoredQuery(t *testing.T) {
	cfg := db{
		StoredStatement: []storedStatement{{Id: "Q", Sql: "SELECT 1"}},
	}

	defer setupTest(t, &cfg, "--db", "env/test.db")(true)

	req := request{
		Transaction: []requestItem{
			{
				Query: "^Q",
			},
		},
	}

	code, _, res := call(t, "http://localhost:12321/test/exec", req)

	require.Equal(t, http.StatusOK, code)

	require.True(t, res.Results[0].Success)
}

const concurrency = 2048

func TestConcurrent(t *testing.T) {
	defer setupTest(t, nil, "--db", "env/test.db")(true)

	req := request{
		Transaction: []requestItem{
			{
				Statement: "CREATE TABLE T1 (ID INT PRIMARY KEY, VAL TEXT NOT NULL)",
			},
		},
	}

	call(t, "http://localhost:12321/test/exec", req)

	req = request{
		Transaction: []requestItem{
			{
				Statement: "INSERT INTO T1 (ID, VAL) VALUES (1, 'ONE')",
			},
			{
				Statement: "INSERT INTO T1 (ID, VAL) VALUES (1, 'TWO')",
				NoFail:    true,
			},
			{
				Query: "SELECT * FROM T1 WHERE ID = 1",
			},
			{
				Statement: "INSERT INTO T1 (ID, VAL) VALUES (:ID, :VAL)",
				Values: mkRaw(map[string]interface{}{
					"ID":  2,
					"VAL": "TWO",
				}),
			},
			{
				Statement: "INSERT INTO T1 (ID, VAL) VALUES (:ID, :VAL)",
				ValuesBatch: []map[string]json.RawMessage{
					mkRaw(map[string]interface{}{
						"ID":  3,
						"VAL": "THREE",
					}),
					mkRaw(map[string]interface{}{
						"ID":  4,
						"VAL": "FOUR",
					})},
			},
			{
				Query: "SELECT * FROM T1 WHERE ID > :ID",
				Values: mkRaw(map[string]interface{}{
					"ID": 0,
				}),
			},
			{
				Statement: "DELETE FROM T1",
			},
		},
	}

	wg := new(sync.WaitGroup)
	wg.Add(concurrency)

	for i := 0; i < concurrency; i++ {
		go func(t *testing.T) {
			defer wg.Done()
			code, _, res := call(t, "http://localhost:12321/test/exec", req)

			require.Equal(t, http.StatusOK, code)

			require.True(t, res.Results[0].Success)
			require.False(t, res.Results[1].Success)
			require.True(t, res.Results[2].Success)
			require.True(t, res.Results[3].Success)
			require.True(t, res.Results[4].Success)
			require.True(t, res.Results[5].Success)
			require.True(t, res.Results[6].Success)

			require.Equal(t, 1, *res.Results[0].RowsUpdated)
			require.Equal(t, "ONE", res.Results[2].ResultSet[0]["VAL"])
			require.Equal(t, 1, *res.Results[3].RowsUpdated)
			require.Equal(t, 2, len(res.Results[4].RowsUpdatedBatch))
			require.Equal(t, 1, res.Results[4].RowsUpdatedBatch[0])
			require.Equal(t, 4, len(res.Results[5].ResultSet))
		}(t)
	}
	wg.Wait()
}

func TestFailRO(t *testing.T) {
	cfg := db{
		ReadOnly: true,
	}

	defer setupTest(t, &cfg, "--db", "env/test.db")(true)

	req := request{
		Transaction: []requestItem{
			{
				Statement: "CREATE TABLE T1 (ID INT PRIMARY KEY, VAL TEXT NOT NULL)",
			},
		},
	}

	code, _, _ := call(t, "http://localhost:12321/test/exec", req)

	require.Equal(t, http.StatusBadRequest, code)
}

func TestOkRO(t *testing.T) {
	closer := setupTest(t, nil, "--db", "env/test.db")

	req := request{
		Transaction: []requestItem{
			{
				Statement: "CREATE TABLE T1 (ID INT PRIMARY KEY, VAL TEXT NOT NULL)",
			},
		},
	}

	call(t, "http://localhost:12321/test/exec", req)

	closer(false)

	cfg := db{
		ReadOnly: true,
	}

	defer setupTest(t, &cfg, "--db", "env/test.db")(true)

	req = request{
		Transaction: []requestItem{
			{
				Query: "SELECT * FROM T1",
			},
		},
	}

	code, _, _ := call(t, "http://localhost:12321/test/exec", req)

	require.Equal(t, http.StatusOK, code)
}

func TestFailSQO(t *testing.T) {
	cfg := db{
		StoredStatement:         []storedStatement{{Id: "Q", Sql: "SELECT 1"}},
		UseOnlyStoredStatements: true,
	}

	defer setupTest(t, &cfg, "--db", "env/test.db")(true)

	req := request{
		Transaction: []requestItem{
			{
				Query: "SELECT 1",
			},
		},
	}

	code, _, _ := call(t, "http://localhost:12321/test/exec", req)

	require.Equal(t, http.StatusBadRequest, code)
}

func TestOkSQO(t *testing.T) {
	cfg := db{
		StoredStatement:         []storedStatement{{Id: "Q", Sql: "SELECT 1"}},
		UseOnlyStoredStatements: true,
	}

	defer setupTest(t, &cfg, "--db", "env/test.db")(true)

	req := request{
		Transaction: []requestItem{
			{
				Query: "^Q",
			},
		},
	}

	code, _, res := call(t, "http://localhost:12321/test/exec", req)

	require.Equal(t, http.StatusOK, code)
	require.True(t, res.Results[0].Success)
}

func TestTxMem(t *testing.T) {
	defer setupTest(t, nil, "--mem-db", "test")(true)
	req := request{
		Transaction: []requestItem{
			{
				Statement: "CREATE TABLE T1 (ID INT PRIMARY KEY, VAL TEXT NOT NULL)",
			},
			{
				Statement: "INSERT INTO T1 (ID, VAL) VALUES (1, 'ONE')",
			},
			{
				Statement: "INSERT INTO T1 (ID, VAL) VALUES (1, 'TWO')",
				NoFail:    true,
			},
			{
				Query: "SELECT * FROM T1 WHERE ID = 1",
			},
			{
				Statement: "INSERT INTO T1 (ID, VAL) VALUES (:ID, :VAL)",
				Values: mkRaw(map[string]interface{}{
					"ID":  2,
					"VAL": "TWO",
				}),
			},
			{
				Statement: "INSERT INTO T1 (ID, VAL) VALUES (:ID, :VAL)",
				ValuesBatch: []map[string]json.RawMessage{
					mkRaw(map[string]interface{}{
						"ID":  3,
						"VAL": "THREE",
					}),
					mkRaw(map[string]interface{}{
						"ID":  4,
						"VAL": "FOUR",
					})},
			},
			{
				Query: "SELECT * FROM T1 WHERE ID > :ID",
				Values: mkRaw(map[string]interface{}{
					"ID": 0,
				}),
			},
		},
	}

	code, _, res := call(t, "http://localhost:12321/test/exec", req)

	require.Equal(t, http.StatusOK, code)

	require.True(t, res.Results[1].Success)
	require.False(t, res.Results[2].Success)
	require.True(t, res.Results[3].Success)
	require.True(t, res.Results[4].Success)
	require.True(t, res.Results[5].Success)
	require.True(t, res.Results[6].Success)

	require.Equal(t, 1, *res.Results[1].RowsUpdated)
	require.Equal(t, "ONE", res.Results[3].ResultSet[0]["VAL"])
	require.Equal(t, 1, *res.Results[4].RowsUpdated)
	require.Equal(t, 2, len(res.Results[5].RowsUpdatedBatch))
	require.Equal(t, 1, res.Results[5].RowsUpdatedBatch[0])
	require.Equal(t, 4, len(res.Results[6].ResultSet))
}

func TestStoredQueryMem(t *testing.T) {
	cfg := db{
		StoredStatement: []storedStatement{{Id: "Q", Sql: "SELECT 1"}},
	}

	defer setupTest(t, &cfg, "--mem-db", "test:env/test.yaml")(true)

	req := request{
		Transaction: []requestItem{
			{
				Query: "^Q",
			},
		},
	}

	code, _, res := call(t, "http://localhost:12321/test/exec", req)

	require.Equal(t, http.StatusOK, code)

	require.True(t, res.Results[0].Success)
}
