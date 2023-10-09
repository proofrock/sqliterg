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
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"strings"
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

func saveCfgToYaml(t *testing.T, cfg *db) {
	os.Remove("env/test.yaml")

	data, err := yaml.Marshal(cfg)
	require.NoError(t, err)

	require.NoError(t, os.WriteFile("env/test.yaml", data, 0600))
}

func setupTest(t *testing.T, cfg *db, printOutput bool, argv ...string) func(bool) {
	if cmd != nil {
		cmd.Process.Kill()
	}
	os.Mkdir("env/backups", 0700)

	if cfg != nil {
		saveCfgToYaml(t, cfg)
	}

	cmd = exec.Command(COMMAND, argv...)
	var outb, errb bytes.Buffer
	cmd.Stdout = &outb
	cmd.Stderr = &errb
	require.NoError(t, cmd.Start())

	time.Sleep(333 * time.Millisecond)

	return func(cleanFiles bool) {
		if printOutput {
			println("== STDOUT ==")
			println(outb.String())
			println("== STDERR ==")
			println(errb.String())
		}
		cmd.Process.Kill()
		cmd = nil
		if cleanFiles {
			os.Remove("env/test.db")
			os.Remove("env/test.db-shm")
			os.Remove("env/test.db-wal")
			os.Remove("env/test1.db")
			os.Remove("env/test1.db-shm")
			os.Remove("env/test1.db-wal")
			os.Remove("env/test2.db")
			os.Remove("env/test2.db-shm")
			os.Remove("env/test2.db-wal")
			os.Remove("env/test.yaml")
			os.RemoveAll("env/backups")
		}
	}
}

func mkNamedParams(mapp map[string]interface{}) map[string]json.RawMessage {
	ret := make(map[string]json.RawMessage)
	for k, v := range mapp {
		bytes, _ := json.Marshal(v)
		ret[k] = bytes
	}
	return ret
}

func mkPositionalParams(arr []interface{}) []json.RawMessage {
	ret := make([]json.RawMessage, len(arr))
	for i, v := range arr {
		bytes, _ := json.Marshal(v)
		ret[i] = bytes
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

func callWithAuth(t *testing.T, url string, req request, username string, password string) (int, string, response) {
	reqbytes, err := json.Marshal(req)
	require.NoError(t, err)
	post, err := http.NewRequest("POST", url, bytes.NewBuffer(reqbytes))
	require.NoError(t, err)
	post.Header.Add("Content-Type", "application/json")
	// Add these lines to set HTTP Basic Authentication
	post.SetBasicAuth(username, password)

	resp, err := http.DefaultClient.Do(post)
	require.NoError(t, err)

	bs, err := io.ReadAll(resp.Body)
	require.NoError(t, err)
	ret := string(bs)
	var obj response
	json.Unmarshal(bs, &obj)

	return resp.StatusCode, ret, obj
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

func TestFileServer(t *testing.T) {
	defer setupTest(t, nil, false, "--serve-dir", ".")(true)

	resp, err := http.Get("http://localhost:12321/env/test.1")
	require.NoError(t, err)

	require.Equal(t, http.StatusOK, resp.StatusCode)
	bs, err := io.ReadAll(resp.Body)
	require.NoError(t, err)
	require.Equal(t, "1", string(bs))
}

func TestMemDbEmpty(t *testing.T) {
	defer setupTest(t, nil, false, "--mem-db", "test")(true)

	code, _, obj := call(t, "http://localhost:12321/test", request{Transaction: []requestItem{{Query: "SELECT 1"}}})

	require.Equal(t, http.StatusOK, code)
	require.Equal(t, 1, len(obj.Results))
	require.True(t, obj.Results[0].Success)
	require.Equal(t, 1, len(obj.Results[0].ResultSet))
	require.Equal(t, 1, len(obj.Results[0].ResultSet[0]))
	require.Equal(t, 1, int(obj.Results[0].ResultSet[0]["1"].(float64)))
}

func TestMemDbEmptyAnotherPort(t *testing.T) {
	defer setupTest(t, nil, false, "--mem-db", "test", "--port", "32123")(true)

	code, _, _ := call(t, "http://localhost:32123/test", request{Transaction: []requestItem{{Query: "SELECT 1"}}})
	require.Equal(t, http.StatusOK, code)
}

func TestMemDbEmptyAnotherBoundIP(t *testing.T) {
	defer setupTest(t, nil, false, "--mem-db", "test", "--bind-host", "1.1.1.1")(true)

	post, err := http.NewRequest("POST", "http://localhost:12321/test", bytes.NewBuffer([]byte{}))
	require.NoError(t, err)
	post.Header.Add("Content-Type", "application/json")

	_, err = http.DefaultClient.Do(post)
	require.Error(t, err)
}

func TestStatementQueryMismatch(t *testing.T) {
	defer setupTest(t, nil, false, "--mem-db", "test")(true)

	code, _, _ := call(t, "http://localhost:12321/test", request{Transaction: []requestItem{{Statement: "SELECT 1"}}})

	require.Equal(t, http.StatusInternalServerError, code)
}

func TestFileDbEmpty(t *testing.T) {
	defer setupTest(t, nil, false, "--db", "env/test.db")(true)

	require.FileExists(t, "env/test.db")

	code, _, obj := call(t, "http://localhost:12321/test", request{Transaction: []requestItem{{Query: "SELECT 1"}}})

	require.Equal(t, http.StatusOK, code)
	require.Equal(t, 1, len(obj.Results))
	require.True(t, obj.Results[0].Success)
	require.Equal(t, 1, len(obj.Results[0].ResultSet))
	require.Equal(t, 1, len(obj.Results[0].ResultSet[0]))
	require.Equal(t, 1, int(obj.Results[0].ResultSet[0]["1"].(float64)))
}

func TestAll3(t *testing.T) {
	defer setupTest(t, nil, false, "--db", "env/test.db", "--mem-db", "test2", "--serve-dir", ".")(true)

	require.FileExists(t, "env/test.db")

	code, _, obj := call(t, "http://localhost:12321/test", request{Transaction: []requestItem{{Query: "SELECT 1"}}})

	require.Equal(t, http.StatusOK, code)
	require.Equal(t, 1, len(obj.Results))
	require.True(t, obj.Results[0].Success)
	require.Equal(t, 1, len(obj.Results[0].ResultSet))
	require.Equal(t, 1, len(obj.Results[0].ResultSet[0]))
	require.Equal(t, 1, int(obj.Results[0].ResultSet[0]["1"].(float64)))

	code, _, obj = call(t, "http://localhost:12321/test", request{Transaction: []requestItem{{Query: "SELECT 1"}}})

	require.Equal(t, http.StatusOK, code)
	require.Equal(t, 1, len(obj.Results))
	require.True(t, obj.Results[0].Success)
	require.Equal(t, 1, len(obj.Results[0].ResultSet))
	require.Equal(t, 1, len(obj.Results[0].ResultSet[0]))
	require.Equal(t, 1, int(obj.Results[0].ResultSet[0]["1"].(float64)))
}

// The following tests are adapted from ws4sqlite

func TestCreate(t *testing.T) {
	defer setupTest(t, nil, false, "--db", "env/test.db")(true)
	req := request{
		Transaction: []requestItem{
			{
				Statement: "CREATE TABLE T1 (ID INT PRIMARY KEY, VAL TEXT NOT NULL)",
			},
		},
	}

	code, _, res := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusOK, code)
	require.True(t, res.Results[0].Success)
}

func TestFail(t *testing.T) {
	defer setupTest(t, nil, false, "--db", "env/test.db")(true)
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

	code, _, _ := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusInternalServerError, code)
}

func TestTx(t *testing.T) {
	defer setupTest(t, nil, false, "--db", "env/test.db")(true)
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
				Values: mkNamedParams(map[string]interface{}{
					"ID":  2,
					"VAL": "TWO",
				}),
			},
			{
				Statement: "INSERT INTO T1 (ID, VAL) VALUES (:ID, :VAL)",
				ValuesBatch: []map[string]json.RawMessage{
					mkNamedParams(map[string]interface{}{
						"ID":  3,
						"VAL": "THREE",
					}),
					mkNamedParams(map[string]interface{}{
						"ID":  4,
						"VAL": "FOUR",
					})},
			},
			{
				Query: "SELECT * FROM T1 WHERE ID > :ID",
				Values: mkNamedParams(map[string]interface{}{
					"ID": 0,
				}),
			},
		},
	}

	code, _, res := call(t, "http://localhost:12321/test", req)

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

func TestTxPositional(t *testing.T) {
	defer setupTest(t, nil, false, "--db", "env/test.db")(true)
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
				Statement: "INSERT INTO T1 (ID, VAL) VALUES (?, ?)",
				Values:    mkPositionalParams([]interface{}{2, "TWO"}),
			},
			{
				Statement: "INSERT INTO T1 (ID, VAL) VALUES (?, ?)",
				ValuesBatch: [][]json.RawMessage{
					mkPositionalParams([]interface{}{3, "THREE"}),
					mkPositionalParams([]interface{}{4, "FOUR"}),
				},
			},
			{
				Query:  "SELECT * FROM T1 WHERE ID > ?",
				Values: mkPositionalParams([]interface{}{0}),
			},
		},
	}

	code, _, res := call(t, "http://localhost:12321/test", req)

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
	defer setupTest(t, nil, false, "--db", "env/test.db")(true)
	req := request{
		Transaction: []requestItem{
			{
				Statement: "CREATE TABLE T1 (ID INT PRIMARY KEY, VAL TEXT NOT NULL)",
			},
		},
	}

	call(t, "http://localhost:12321/test", req)

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

	code, _, _ := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusInternalServerError, code)

	req = request{
		Transaction: []requestItem{
			{
				Query: "SELECT * FROM T1",
			},
		},
	}

	code, _, res := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusOK, code)

	require.True(t, res.Results[0].Success)
	require.Equal(t, 0, len(res.Results[0].ResultSet))
}

func TestStoredQuery(t *testing.T) {
	cfg := db{
		StoredStatement: []storedStatement{{Id: "Q", Sql: "SELECT 1"}},
	}

	defer setupTest(t, &cfg, false, "--db", "env/test.db")(true)

	req := request{
		Transaction: []requestItem{
			{
				Query: "^Q",
			},
		},
	}

	code, _, res := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusOK, code)

	require.True(t, res.Results[0].Success)
}

const concurrency = 2048

func TestConcurrent(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping testing in short mode")
	}

	defer setupTest(t, nil, false, "--db", "env/test.db")(true)

	req := request{
		Transaction: []requestItem{
			{
				Statement: "CREATE TABLE T1 (ID INT PRIMARY KEY, VAL TEXT NOT NULL)",
			},
		},
	}

	call(t, "http://localhost:12321/test", req)

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
				Values: mkNamedParams(map[string]interface{}{
					"ID":  2,
					"VAL": "TWO",
				}),
			},
			{
				Statement: "INSERT INTO T1 (ID, VAL) VALUES (:ID, :VAL)",
				ValuesBatch: []map[string]json.RawMessage{
					mkNamedParams(map[string]interface{}{
						"ID":  3,
						"VAL": "THREE",
					}),
					mkNamedParams(map[string]interface{}{
						"ID":  4,
						"VAL": "FOUR",
					})},
			},
			{
				Query: "SELECT * FROM T1 WHERE ID > :ID",
				Values: mkNamedParams(map[string]interface{}{
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
			code, _, res := call(t, "http://localhost:12321/test", req)

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

	defer setupTest(t, &cfg, false, "--db", "env/test.db")(true)

	req := request{
		Transaction: []requestItem{
			{
				Statement: "CREATE TABLE T1 (ID INT PRIMARY KEY, VAL TEXT NOT NULL)",
			},
		},
	}

	code, _, _ := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusInternalServerError, code)
}

func TestOkRO(t *testing.T) {
	closer := setupTest(t, nil, false, "--db", "env/test.db")

	req := request{
		Transaction: []requestItem{
			{
				Statement: "CREATE TABLE T1 (ID INT PRIMARY KEY, VAL TEXT NOT NULL)",
			},
		},
	}

	call(t, "http://localhost:12321/test", req)

	closer(false)

	cfg := db{
		ReadOnly: true,
	}

	defer setupTest(t, &cfg, false, "--db", "env/test.db")(true)

	req = request{
		Transaction: []requestItem{
			{
				Query: "SELECT * FROM T1",
			},
		},
	}

	code, _, _ := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusOK, code)
}

func TestFailSQO(t *testing.T) {
	cfg := db{
		StoredStatement:         []storedStatement{{Id: "Q", Sql: "SELECT 1"}},
		UseOnlyStoredStatements: true,
	}

	defer setupTest(t, &cfg, false, "--db", "env/test.db")(true)

	req := request{
		Transaction: []requestItem{
			{
				Query: "SELECT 1",
			},
		},
	}

	code, _, _ := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusConflict, code)
}

func TestOkSQO(t *testing.T) {
	cfg := db{
		StoredStatement:         []storedStatement{{Id: "Q", Sql: "SELECT 1"}},
		UseOnlyStoredStatements: true,
	}

	defer setupTest(t, &cfg, false, "--db", "env/test.db")(true)

	req := request{
		Transaction: []requestItem{
			{
				Query: "^Q",
			},
		},
	}

	code, _, res := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusOK, code)
	require.True(t, res.Results[0].Success)
}

func TestTxMem(t *testing.T) {
	defer setupTest(t, nil, false, "--mem-db", "test")(true)
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
				Values: mkNamedParams(map[string]interface{}{
					"ID":  2,
					"VAL": "TWO",
				}),
			},
			{
				Statement: "INSERT INTO T1 (ID, VAL) VALUES (:ID, :VAL)",
				ValuesBatch: []map[string]json.RawMessage{
					mkNamedParams(map[string]interface{}{
						"ID":  3,
						"VAL": "THREE",
					}),
					mkNamedParams(map[string]interface{}{
						"ID":  4,
						"VAL": "FOUR",
					})},
			},
			{
				Query: "SELECT * FROM T1 WHERE ID > :ID",
				Values: mkNamedParams(map[string]interface{}{
					"ID": 0,
				}),
			},
		},
	}

	code, _, res := call(t, "http://localhost:12321/test", req)

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

	defer setupTest(t, &cfg, false, "--mem-db", "test::env/test.yaml")(true)

	req := request{
		Transaction: []requestItem{
			{
				Query: "^Q",
			},
		},
	}

	code, _, res := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusOK, code)

	require.True(t, res.Results[0].Success)
}

var TRUE bool = true

func TestInitMacro(t *testing.T) {
	cfg := db{
		Macros: []macro{{Id: "M1", Statements: []string{"CREATE TABLE T1 (ID INT PRIMARY KEY, VAL TEXT NOT NULL)"}, Execution: execution{OnCreate: &TRUE}}},
	}

	defer setupTest(t, &cfg, false, "--mem-db", "test::env/test.yaml")(true)

	req := request{
		Transaction: []requestItem{
			{
				Statement: "INSERT INTO T1 VALUES(1, 'ONE')",
			},
		},
	}

	code, _, res := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusOK, code)

	require.True(t, res.Results[0].Success)
}

func TestStartupMacroIsNotCreate(t *testing.T) {
	cfg := db{
		Macros: []macro{
			{
				Id: "M1",
				Statements: []string{
					"CREATE TABLE IF NOT EXISTS T1 (ID INT, VAL TEXT)",
					"DELETE FROM T1",
				},
				Execution: execution{
					OnCreate: &TRUE,
				},
			},
			{
				Id: "M2",
				Statements: []string{
					"INSERT INTO T1 VALUES (1, '')",
				},
				Execution: execution{
					OnStartup: &TRUE,
				},
			},
		},
	}

	closer := setupTest(t, &cfg, false, "--db", "env/test.db")

	req := request{
		Transaction: []requestItem{
			{
				Query: "SELECT COUNT(1) as CNT FROM T1",
			},
		},
	}

	code, _, _ := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusOK, code)

	closer(false)

	defer setupTest(t, &cfg, false, "--db", "env/test.db")(true)

	code, _, res := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusOK, code)

	require.Equal(t, float64(2), res.Results[0].ResultSet[0]["CNT"])
}

func TestStartupAndCreateMacroIsJustOne(t *testing.T) {
	cfg := db{
		Macros: []macro{{Id: "M1", Statements: []string{"CREATE TABLE IF NOT EXISTS T1 (ID INT, VAL TEXT)", "INSERT INTO T1 VALUES (1, '')"}, Execution: execution{OnCreate: &TRUE, OnStartup: &TRUE}}},
	}

	defer setupTest(t, &cfg, false, "--mem-db", "test::env/test.yaml")(true)

	req := request{
		Transaction: []requestItem{
			{
				Query: "SELECT COUNT(1) as CNT FROM T1",
			},
		},
	}

	code, _, res := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusOK, code)

	require.Equal(t, float64(1), res.Results[0].ResultSet[0]["CNT"])
}

func TestInitMacroFailureDeletesFile(t *testing.T) {
	cfg := db{
		Macros: []macro{{Id: "M1", Statements: []string{"<INVALID SQL> CREATE TABLE T1 (ID INT PRIMARY KEY, VAL TEXT NOT NULL)"}, Execution: execution{OnCreate: &TRUE}}},
	}

	defer setupTest(t, &cfg, false, "--mem-db", "test::env/test.yaml")(true)

	require.NoFileExists(t, "env/test.db")
}

func TestInitMacroReferencingStoredStatement(t *testing.T) {
	cfg := db{
		StoredStatement: []storedStatement{{Id: "SQL", Sql: "CREATE TABLE T1 (ID INT PRIMARY KEY, VAL TEXT NOT NULL)"}},
		Macros:          []macro{{Id: "M1", Statements: []string{"^SQL"}, Execution: execution{OnCreate: &TRUE}}},
	}

	defer setupTest(t, &cfg, false, "--mem-db", "test::env/test.yaml")(true)

	req := request{
		Transaction: []requestItem{
			{
				Statement: "INSERT INTO T1 VALUES(1, 'ONE')",
			},
		},
	}

	code, _, res := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusOK, code)

	require.True(t, res.Results[0].Success)
}

var PERIOD uint = 1

func TestPeriodicMacro(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping testing in short mode")
	}

	cfg := db{
		Macros: []macro{
			{
				Id: "M1",
				Statements: []string{
					"CREATE TABLE IF NOT EXISTS T1 (ID INT, VAL TEXT)",
				},
				Execution: execution{
					OnCreate: &TRUE,
				},
			},
			{
				Id: "M2",
				Statements: []string{
					"INSERT INTO T1 VALUES (1, '')",
				},
				Execution: execution{
					Period: &PERIOD,
				},
			},
		},
	}

	setupTest(t, &cfg, false, "--db", "env/test.db")(false)

	defer setupTest(t, &cfg, false, "--db", "env/test.db")(true)

	time.Sleep(70 * time.Second)

	req := request{
		Transaction: []requestItem{
			{
				Query: "SELECT COUNT(1) as CNT FROM T1",
			},
		},
	}

	code, _, res := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusOK, code)

	require.Equal(t, float64(1), res.Results[0].ResultSet[0]["CNT"])
}

func TestCallableMacro(t *testing.T) {
	cfg := db{
		Macros: []macro{
			{
				Id: "M1",
				Statements: []string{
					"CREATE TABLE IF NOT EXISTS T1 (ID INT, VAL TEXT)",
				},
				Execution: execution{
					OnCreate: &TRUE,
				},
			},
			{
				Id: "M2",
				Statements: []string{
					"INSERT INTO T1 VALUES (1, '')",
				},
				Execution: execution{
					WebService: &webService{},
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--db", "env/test.db")(true)

	req := request{
		Transaction: []requestItem{},
	}

	code, _, _ := call(t, "http://localhost:12321/test/macro/M2", req)

	require.Equal(t, http.StatusOK, code)

	code, _, _ = call(t, "http://localhost:12321/test/macro/M2", req)

	require.Equal(t, http.StatusOK, code)

	req = request{
		Transaction: []requestItem{
			{
				Query: "SELECT COUNT(1) as CNT FROM T1",
			},
		},
	}

	code, _, res := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusOK, code)

	require.Equal(t, float64(2), res.Results[0].ResultSet[0]["CNT"])
}

var ciao string = "ciao"
var hciao string = "b133a0c0e9bee3be20163d2ad31d6248db292aa6dcb1ee087a2aa50e0fc75ae2"
var custAuthError = 499

func TestCallableMacroNormalPassword(t *testing.T) {
	cfg := db{
		Macros: []macro{
			{
				Id: "M1",
				Statements: []string{
					"CREATE TABLE IF NOT EXISTS T1 (ID INT, VAL TEXT)",
				},
				Execution: execution{
					OnCreate: &TRUE,
				},
			},
			{
				Id: "M2",
				Statements: []string{
					"INSERT INTO T1 VALUES (1, '')",
				},
				Execution: execution{
					WebService: &webService{
						AuthToken: &ciao,
					},
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--db", "env/test.db")(true)

	req := request{
		Transaction: []requestItem{},
	}

	code, _, _ := call(t, "http://localhost:12321/test/macro/M2?token=ciao", req)

	require.Equal(t, http.StatusOK, code)

	req = request{
		Transaction: []requestItem{
			{
				Query: "SELECT COUNT(1) as CNT FROM T1",
			},
		},
	}

	code, _, res := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusOK, code)

	require.Equal(t, float64(1), res.Results[0].ResultSet[0]["CNT"])
}
func TestCallableMacroHashedPassword(t *testing.T) {
	cfg := db{
		Macros: []macro{
			{
				Id: "M1",
				Statements: []string{
					"CREATE TABLE IF NOT EXISTS T1 (ID INT, VAL TEXT)",
				},
				Execution: execution{
					OnCreate: &TRUE,
				},
			},
			{
				Id: "M2",
				Statements: []string{
					"INSERT INTO T1 VALUES (1, '')",
				},
				Execution: execution{
					WebService: &webService{
						HashedAuthToken: &hciao,
					},
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--db", "env/test.db")(true)

	req := request{
		Transaction: []requestItem{},
	}

	code, _, _ := call(t, "http://localhost:12321/test/macro/M2?token=ciao", req)

	require.Equal(t, http.StatusOK, code)

	req = request{
		Transaction: []requestItem{
			{
				Query: "SELECT COUNT(1) as CNT FROM T1",
			},
		},
	}

	code, _, res := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusOK, code)

	require.Equal(t, float64(1), res.Results[0].ResultSet[0]["CNT"])
}

func TestCallableMacroAuthFail(t *testing.T) {
	cfg := db{
		Macros: []macro{
			{
				Id: "M2",
				Statements: []string{
					"INSERT INTO T1 VALUES (1, '')",
				},
				Execution: execution{
					WebService: &webService{
						AuthToken: &ciao,
					},
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--db", "env/test.db")(true)

	req := request{
		Transaction: []requestItem{},
	}

	code, _, _ := call(t, "http://localhost:12321/test/macro/M2", req)

	require.Equal(t, http.StatusUnauthorized, code)
}

func TestCallableMacroAuthFailCustomError(t *testing.T) {
	cfg := db{
		Macros: []macro{
			{
				Id: "M2",
				Statements: []string{
					"INSERT INTO T1 VALUES (1, '')",
				},
				Execution: execution{
					WebService: &webService{
						AuthErrorCode: &custAuthError,
						AuthToken:     &ciao,
					},
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--db", "env/test.db")(true)

	req := request{
		Transaction: []requestItem{},
	}

	code, _, _ := call(t, "http://localhost:12321/test/macro/M2", req)

	require.Equal(t, custAuthError, code)
}

func now() string {
	return time.Now().Format("20060102-1504")
}

func TestInitBackup(t *testing.T) {
	cfg := db{
		Backup: backup{
			BackupDir: "env/backups",
			NumFiles:  1,
			Execution: execution{
				OnCreate: &TRUE,
			},
		},
	}

	defer setupTest(t, &cfg, false, "--mem-db", "test::env/test.yaml")(true)

	require.FileExists(t, fmt.Sprintf("env/backups/test_%s", now()))
}

func TestNoInitBackup(t *testing.T) {
	setupTest(t, nil, false, "--db", "env/test.db")

	cfg := db{
		Backup: backup{
			BackupDir: "env/backups",
			NumFiles:  1,
			Execution: execution{
				OnCreate: &TRUE,
			},
		},
	}

	defer setupTest(t, &cfg, false, "--mem-db", "test::env/test.yaml")(true)

	require.NoFileExists(t, fmt.Sprintf("env/backups/test_%s.db", now()))
}

func TestStartupBackup1(t *testing.T) {
	cfg := db{
		Backup: backup{
			BackupDir: "env/backups",
			NumFiles:  1,
			Execution: execution{
				OnStartup: &TRUE,
			},
		},
	}

	defer setupTest(t, &cfg, false, "--db", "env/test.db")(true)

	require.FileExists(t, fmt.Sprintf("env/backups/test_%s.db", now()))
}

func TestStartupBackup2(t *testing.T) {
	closer := setupTest(t, nil, false, "--db", "env/test.db")

	cfg := db{
		Backup: backup{
			BackupDir: "env/backups",
			NumFiles:  1,
			Execution: execution{
				OnStartup: &TRUE,
			},
		},
	}

	closer(false)

	defer setupTest(t, &cfg, false, "--db", "env/test.db")(true)

	require.FileExists(t, fmt.Sprintf("env/backups/test_%s.db", now()))
}

func TestPeriodicBackupWith1File(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping testing in short mode")
	}

	cfg := db{
		Backup: backup{
			BackupDir: "env/backups",
			NumFiles:  1,
			Execution: execution{
				OnCreate: &TRUE,
				Period:   &PERIOD,
			},
		},
	}

	defer setupTest(t, &cfg, false, "--db", "env/test.db")(true)

	// It can fail even if it's correct, because of timing (2/60~=3% probability)
	time.Sleep(2 * time.Second)
	bkp1 := fmt.Sprintf("env/backups/test_%s.db", now())
	// require.FileExists(t, bkp1)

	time.Sleep(60 * time.Second)
	bkp2 := fmt.Sprintf("env/backups/test_%s.db", now())
	require.NoFileExists(t, bkp1)
	require.FileExists(t, bkp2)
}

func TestCallableBackup(t *testing.T) {
	cfg := db{
		Backup: backup{
			BackupDir: "env/backups",
			NumFiles:  1,
			Execution: execution{
				WebService: &webService{},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--db", "env/test.db")(true)

	req := request{
		Transaction: []requestItem{},
	}

	code, _, _ := call(t, "http://localhost:12321/test/backup", req)

	require.Equal(t, http.StatusOK, code)

	require.FileExists(t, fmt.Sprintf("env/backups/test_%s.db", now()))
}

func TestCallableBackupAuthOk(t *testing.T) {
	cfg := db{
		Backup: backup{
			BackupDir: "env/backups",
			NumFiles:  1,
			Execution: execution{
				WebService: &webService{
					AuthToken: &ciao,
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--db", "env/test.db")(true)

	req := request{
		Transaction: []requestItem{},
	}

	code, _, _ := call(t, "http://localhost:12321/test/backup?token=ciao", req)

	require.Equal(t, http.StatusOK, code)

	require.FileExists(t, fmt.Sprintf("env/backups/test_%s.db", now()))
}

func TestCallableBackupAuthKo(t *testing.T) {
	cfg := db{
		Backup: backup{
			BackupDir: "env/backups",
			NumFiles:  1,
			Execution: execution{
				WebService: &webService{
					AuthToken: &ciao,
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--db", "env/test.db")(true)

	req := request{
		Transaction: []requestItem{},
	}

	code, _, _ := call(t, "http://localhost:12321/test/backup?token=cibo", req)

	require.Equal(t, http.StatusUnauthorized, code)
}

func TestCallableBackupAuthOkHash(t *testing.T) {
	cfg := db{
		Backup: backup{
			BackupDir: "env/backups",
			NumFiles:  1,
			Execution: execution{
				WebService: &webService{
					HashedAuthToken: &hciao,
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--db", "env/test.db")(true)

	req := request{
		Transaction: []requestItem{},
	}

	code, _, _ := call(t, "http://localhost:12321/test/backup?token=ciao", req)

	require.Equal(t, http.StatusOK, code)

	require.FileExists(t, fmt.Sprintf("env/backups/test_%s.db", now()))
}

func TestCallableBackupAuthKoHash(t *testing.T) {
	cfg := db{
		Backup: backup{
			BackupDir: "env/backups",
			NumFiles:  1,
			Execution: execution{
				WebService: &webService{
					HashedAuthToken: &hciao,
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--db", "env/test.db")(true)

	req := request{
		Transaction: []requestItem{},
	}

	code, _, _ := call(t, "http://localhost:12321/test/backup?token=cibo", req)

	require.Equal(t, http.StatusUnauthorized, code)
}

func TestCallableBackupAuthKoCustomError(t *testing.T) {
	cfg := db{
		Backup: backup{
			BackupDir: "env/backups",
			NumFiles:  1,
			Execution: execution{
				WebService: &webService{
					AuthErrorCode: &custAuthError,
					AuthToken:     &ciao,
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--db", "env/test.db")(true)

	req := request{
		Transaction: []requestItem{},
	}

	code, _, _ := call(t, "http://localhost:12321/test/backup?token=cibo", req)

	require.Equal(t, custAuthError, code)
}

func TestFailROButMacroCanModify(t *testing.T) {
	cfg := db{
		ReadOnly: true,
		Macros: []macro{
			{
				Id: "M1",
				Statements: []string{
					// if it failed, it would give connection error later on
					"CREATE TABLE T2 (ID INT PRIMARY KEY, VAL TEXT NOT NULL)",
				},
				Execution: execution{
					OnCreate: &TRUE,
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--db", "env/test.db")(true)

	req := request{
		Transaction: []requestItem{
			{
				Statement: "CREATE TABLE T2 (ID INT PRIMARY KEY, VAL TEXT NOT NULL)",
			},
		},
	}

	code, _, _ := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusInternalServerError, code)
}

func TestFailROButMacroCanModify2(t *testing.T) {
	cfg := db{
		ReadOnly: true,
		Macros: []macro{
			{
				Id: "M1",
				Statements: []string{
					// if it failed, it would give connection error later on
					"CREATE TABLE T2 (ID INT PRIMARY KEY, VAL TEXT NOT NULL)",
				},
				Execution: execution{
					OnStartup: &TRUE,
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--db", "env/test.db")(true)

	req := request{
		Transaction: []requestItem{
			{
				Statement: "CREATE TABLE T2 (ID INT PRIMARY KEY, VAL TEXT NOT NULL)",
			},
		},
	}

	code, _, _ := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusInternalServerError, code)
}

func TestDbSegregationMem(t *testing.T) {
	defer setupTest(t, nil, false, "--mem-db", "test1", "--mem-db", "test2")(true)
	req := request{
		Transaction: []requestItem{
			{
				Statement: "CREATE TABLE T1 (ID INT PRIMARY KEY, VAL TEXT NOT NULL)",
			},
		},
	}

	code, _, res := call(t, "http://localhost:12321/test1", req)

	require.Equal(t, http.StatusOK, code)
	require.True(t, res.Results[0].Success)

	code, _, res = call(t, "http://localhost:12321/test2", req)

	require.Equal(t, http.StatusOK, code)
	require.True(t, res.Results[0].Success)
}

func TestDbSegregationFile(t *testing.T) {
	defer setupTest(t, nil, false, "--db", "env/test1.db", "--db", "env/test2.db")(true)
	req := request{
		Transaction: []requestItem{
			{
				Statement: "CREATE TABLE T1 (ID INT PRIMARY KEY, VAL TEXT NOT NULL)",
			},
		},
	}

	code, _, res := call(t, "http://localhost:12321/test1", req)

	require.Equal(t, http.StatusOK, code)
	require.True(t, res.Results[0].Success)

	code, _, res = call(t, "http://localhost:12321/test2", req)

	require.Equal(t, http.StatusOK, code)
	require.True(t, res.Results[0].Success)
}

func TestDbSegregationFileAndMem(t *testing.T) {
	defer setupTest(t, nil, false, "--db", "env/test1.db", "--mem-db", "test2")(true)
	req := request{
		Transaction: []requestItem{
			{
				Statement: "CREATE TABLE T1 (ID INT PRIMARY KEY, VAL TEXT NOT NULL)",
			},
		},
	}

	code, _, res := call(t, "http://localhost:12321/test1", req)

	require.Equal(t, http.StatusOK, code)
	require.True(t, res.Results[0].Success)

	code, _, res = call(t, "http://localhost:12321/test2", req)

	require.Equal(t, http.StatusOK, code)
	require.True(t, res.Results[0].Success)
}

func TestProfilerPayloadOnFile(t *testing.T) {
	cfg := db{
		Auth: &authr{
			Mode:    "INLINE",
			ByQuery: "SELECT 1 FROM AUTH WHERE USER = :user AND PASS = :password",
		},
		Macros: []macro{
			{
				Id: "M1",
				Statements: []string{
					"CREATE TABLE IF NOT EXISTS TBL (ID INT, VAL TEXT)",
					"CREATE TABLE IF NOT EXISTS AUTH (USER TEXT, PASS TEXT)",
					"DELETE FROM AUTH",
					"INSERT INTO AUTH VALUES ('myUser', 'ciao')",
				},
				Execution: execution{
					OnCreate: &TRUE,
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--db", "env/test.db")(true)
	req := request{
		Credentials: &credentials{
			User:     "myUser",
			Password: "ciao",
		},
		Transaction: []requestItem{
			{
				Statement: "DELETE FROM TBL",
			},
			{
				Query: "SELECT * FROM TBL",
			},
			{
				Statement: "INSERT INTO TBL (ID, VAL) VALUES (:id, :val)",
				Values:    mkNamedParams(map[string]interface{}{"id": 0, "val": "zero"}),
			},
			{
				Statement: "INSERT INTO TBL (ID, VAL) VALUES (:id, :val)",
				ValuesBatch: []map[string]json.RawMessage{
					mkNamedParams(map[string]interface{}{"id": 1, "val": "uno"}),
					mkNamedParams(map[string]interface{}{"id": 2, "val": "due"}),
				},
			},
			{
				NoFail:    true,
				Statement: "INSERT INTO TBL (ID, VAL) VALUES (:id, :val, 1)",
				ValuesBatch: []map[string]json.RawMessage{
					mkNamedParams(map[string]interface{}{"id": 1, "val": "uno"}),
					mkNamedParams(map[string]interface{}{"id": 2, "val": "due"}),
				},
			},
			{
				Statement: "INSERT INTO TBL (ID, VAL) VALUES (:id, :val)",
				ValuesBatch: []map[string]json.RawMessage{
					mkNamedParams(map[string]interface{}{"id": 3, "val": "tre"}),
				},
			},
			{
				Query:  "SELECT * FROM TBL WHERE ID=:id",
				Values: mkNamedParams(map[string]interface{}{"id": 1}),
			},
			{
				Statement: "DELETE FROM TBL",
			},
		},
	}

	code, _, res := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusOK, code)

	require.True(t, res.Results[1].Success)
	require.True(t, res.Results[2].Success)
	require.True(t, res.Results[3].Success)
	require.False(t, res.Results[4].Success)
	require.True(t, res.Results[5].Success)
	require.True(t, res.Results[6].Success)

	require.Equal(t, 0, *res.Results[0].RowsUpdated)
	require.Equal(t, 0, len(res.Results[1].ResultSet))
	require.Equal(t, 1, *res.Results[2].RowsUpdated)
	require.Equal(t, 2, len(res.Results[3].RowsUpdatedBatch))
	require.Equal(t, 1, res.Results[3].RowsUpdatedBatch[0])
	require.Equal(t, 1, res.Results[3].RowsUpdatedBatch[1])
	require.Equal(t, 1, len(res.Results[5].RowsUpdatedBatch))
	require.Equal(t, 1, res.Results[5].RowsUpdatedBatch[0])
	require.Equal(t, 1, len(res.Results[6].ResultSet))
	require.Equal(t, 4, *res.Results[7].RowsUpdated)

	code, _, res = call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusOK, code)

	require.True(t, res.Results[1].Success)
	require.True(t, res.Results[2].Success)
	require.True(t, res.Results[3].Success)
	require.False(t, res.Results[4].Success)
	require.True(t, res.Results[5].Success)
	require.True(t, res.Results[6].Success)

	require.Equal(t, 0, *res.Results[0].RowsUpdated)
	require.Equal(t, 0, len(res.Results[1].ResultSet))
	require.Equal(t, 1, *res.Results[2].RowsUpdated)
	require.Equal(t, 2, len(res.Results[3].RowsUpdatedBatch))
	require.Equal(t, 1, res.Results[3].RowsUpdatedBatch[0])
	require.Equal(t, 1, res.Results[3].RowsUpdatedBatch[1])
	require.Equal(t, 1, len(res.Results[5].RowsUpdatedBatch))
	require.Equal(t, 1, res.Results[5].RowsUpdatedBatch[0])
	require.Equal(t, 1, len(res.Results[6].ResultSet))
	require.Equal(t, 4, *res.Results[7].RowsUpdated)
}

func TestProfilerPayloadOnMem(t *testing.T) {
	cfg := db{
		Auth: &authr{
			Mode:    "INLINE",
			ByQuery: "SELECT 1 FROM AUTH WHERE USER = :user AND PASS = :password",
		},
		Macros: []macro{
			{
				Id: "M1",
				Statements: []string{
					"CREATE TABLE IF NOT EXISTS TBL (ID INT, VAL TEXT)",
					"CREATE TABLE IF NOT EXISTS AUTH (USER TEXT, PASS TEXT)",
					"DELETE FROM AUTH",
					"INSERT INTO AUTH VALUES ('myUser', 'ciao')",
				},
				Execution: execution{
					OnCreate: &TRUE,
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--mem-db", "test::env/test.yaml")(true)
	req := request{
		Credentials: &credentials{
			User:     "myUser",
			Password: "ciao",
		},
		Transaction: []requestItem{
			{
				Statement: "DELETE FROM TBL",
			},
			{
				Query: "SELECT * FROM TBL",
			},
			{
				Statement: "INSERT INTO TBL (ID, VAL) VALUES (:id, :val)",
				Values:    mkNamedParams(map[string]interface{}{"id": 0, "val": "zero"}),
			},
			{
				Statement: "INSERT INTO TBL (ID, VAL) VALUES (:id, :val)",
				ValuesBatch: []map[string]json.RawMessage{
					mkNamedParams(map[string]interface{}{"id": 1, "val": "uno"}),
					mkNamedParams(map[string]interface{}{"id": 2, "val": "due"}),
				},
			},
			{
				NoFail:    true,
				Statement: "INSERT INTO TBL (ID, VAL) VALUES (:id, :val, 1)",
				ValuesBatch: []map[string]json.RawMessage{
					mkNamedParams(map[string]interface{}{"id": 1, "val": "uno"}),
					mkNamedParams(map[string]interface{}{"id": 2, "val": "due"}),
				},
			},
			{
				Statement: "INSERT INTO TBL (ID, VAL) VALUES (:id, :val)",
				ValuesBatch: []map[string]json.RawMessage{
					mkNamedParams(map[string]interface{}{"id": 3, "val": "tre"}),
				},
			},
			{
				Query:  "SELECT * FROM TBL WHERE ID=:id",
				Values: mkNamedParams(map[string]interface{}{"id": 1}),
			},
			{
				Statement: "DELETE FROM TBL",
			},
		},
	}

	code, _, res := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusOK, code)

	require.True(t, res.Results[1].Success)
	require.True(t, res.Results[2].Success)
	require.True(t, res.Results[3].Success)
	require.False(t, res.Results[4].Success)
	require.True(t, res.Results[5].Success)
	require.True(t, res.Results[6].Success)

	require.Equal(t, 0, *res.Results[0].RowsUpdated)
	require.Equal(t, 0, len(res.Results[1].ResultSet))
	require.Equal(t, 1, *res.Results[2].RowsUpdated)
	require.Equal(t, 2, len(res.Results[3].RowsUpdatedBatch))
	require.Equal(t, 1, res.Results[3].RowsUpdatedBatch[0])
	require.Equal(t, 1, res.Results[3].RowsUpdatedBatch[1])
	require.Equal(t, 1, len(res.Results[5].RowsUpdatedBatch))
	require.Equal(t, 1, res.Results[5].RowsUpdatedBatch[0])
	require.Equal(t, 1, len(res.Results[6].ResultSet))
	require.Equal(t, 4, *res.Results[7].RowsUpdated)

	code, _, res = call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusOK, code)

	require.True(t, res.Results[1].Success)
	require.True(t, res.Results[2].Success)
	require.True(t, res.Results[3].Success)
	require.False(t, res.Results[4].Success)
	require.True(t, res.Results[5].Success)
	require.True(t, res.Results[6].Success)

	require.Equal(t, 0, *res.Results[0].RowsUpdated)
	require.Equal(t, 0, len(res.Results[1].ResultSet))
	require.Equal(t, 1, *res.Results[2].RowsUpdated)
	require.Equal(t, 2, len(res.Results[3].RowsUpdatedBatch))
	require.Equal(t, 1, res.Results[3].RowsUpdatedBatch[0])
	require.Equal(t, 1, res.Results[3].RowsUpdatedBatch[1])
	require.Equal(t, 1, len(res.Results[5].RowsUpdatedBatch))
	require.Equal(t, 1, res.Results[5].RowsUpdatedBatch[0])
	require.Equal(t, 1, len(res.Results[6].ResultSet))
	require.Equal(t, 4, *res.Results[7].RowsUpdated)
}

func TestJournalMode(t *testing.T) {
	cfg := db{
		Auth: &authr{
			Mode:    "INLINE",
			ByQuery: "SELECT 1 FROM AUTH WHERE USER = :user AND PASS = :password",
		},
		JournalMode: "DELETE",
		Macros: []macro{
			{
				Id: "M1",
				Statements: []string{
					"CREATE TABLE IF NOT EXISTS TBL (ID INT, VAL TEXT)",
					"CREATE TABLE IF NOT EXISTS AUTH (USER TEXT, PASS TEXT)",
					"DELETE FROM AUTH",
					"INSERT INTO AUTH VALUES ('myUser', 'ciao')",
				},
				Execution: execution{
					OnCreate: &TRUE,
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--db", "env/test.db")(true)
	req := request{
		Credentials: &credentials{
			User:     "myUser",
			Password: "ciao",
		},
		Transaction: []requestItem{
			{
				Statement: "DELETE FROM TBL",
			},
			{
				Query: "SELECT * FROM TBL",
			},
			{
				Statement: "INSERT INTO TBL (ID, VAL) VALUES (:id, :val)",
				Values:    mkNamedParams(map[string]interface{}{"id": 0, "val": "zero"}),
			},
			{
				Statement: "INSERT INTO TBL (ID, VAL) VALUES (:id, :val)",
				ValuesBatch: []map[string]json.RawMessage{
					mkNamedParams(map[string]interface{}{"id": 1, "val": "uno"}),
					mkNamedParams(map[string]interface{}{"id": 2, "val": "due"}),
				},
			},
			{
				NoFail:    true,
				Statement: "INSERT INTO TBL (ID, VAL) VALUES (:id, :val, 1)",
				ValuesBatch: []map[string]json.RawMessage{
					mkNamedParams(map[string]interface{}{"id": 1, "val": "uno"}),
					mkNamedParams(map[string]interface{}{"id": 2, "val": "due"}),
				},
			},
			{
				Statement: "INSERT INTO TBL (ID, VAL) VALUES (:id, :val)",
				ValuesBatch: []map[string]json.RawMessage{
					mkNamedParams(map[string]interface{}{"id": 3, "val": "tre"}),
				},
			},
			{
				Query:  "SELECT * FROM TBL WHERE ID=:id",
				Values: mkNamedParams(map[string]interface{}{"id": 1}),
			},
			{
				Statement: "DELETE FROM TBL",
			},
		},
	}

	code, _, res := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusOK, code)

	require.True(t, res.Results[1].Success)
	require.True(t, res.Results[2].Success)
	require.True(t, res.Results[3].Success)
	require.False(t, res.Results[4].Success)
	require.True(t, res.Results[5].Success)
	require.True(t, res.Results[6].Success)

	require.Equal(t, 0, *res.Results[0].RowsUpdated)
	require.Equal(t, 0, len(res.Results[1].ResultSet))
	require.Equal(t, 1, *res.Results[2].RowsUpdated)
	require.Equal(t, 2, len(res.Results[3].RowsUpdatedBatch))
	require.Equal(t, 1, res.Results[3].RowsUpdatedBatch[0])
	require.Equal(t, 1, res.Results[3].RowsUpdatedBatch[1])
	require.Equal(t, 1, len(res.Results[5].RowsUpdatedBatch))
	require.Equal(t, 1, res.Results[5].RowsUpdatedBatch[0])
	require.Equal(t, 1, len(res.Results[6].ResultSet))
	require.Equal(t, 4, *res.Results[7].RowsUpdated)

	code, _, res = call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusOK, code)

	require.True(t, res.Results[1].Success)
	require.True(t, res.Results[2].Success)
	require.True(t, res.Results[3].Success)
	require.False(t, res.Results[4].Success)
	require.True(t, res.Results[5].Success)
	require.True(t, res.Results[6].Success)

	require.Equal(t, 0, *res.Results[0].RowsUpdated)
	require.Equal(t, 0, len(res.Results[1].ResultSet))
	require.Equal(t, 1, *res.Results[2].RowsUpdated)
	require.Equal(t, 2, len(res.Results[3].RowsUpdatedBatch))
	require.Equal(t, 1, res.Results[3].RowsUpdatedBatch[0])
	require.Equal(t, 1, res.Results[3].RowsUpdatedBatch[1])
	require.Equal(t, 1, len(res.Results[5].RowsUpdatedBatch))
	require.Equal(t, 1, res.Results[5].RowsUpdatedBatch[0])
	require.Equal(t, 1, len(res.Results[6].ResultSet))
	require.Equal(t, 4, *res.Results[7].RowsUpdated)
}

func TestExplicitYAML(t *testing.T) {
	cfg := db{
		JournalMode: "DELETE",
	}

	defer setupTest(t, &cfg, false, "--db", "env/test.db::/env/test.yaml")(true)
	req := request{
		Transaction: []requestItem{
			{
				Query: "SELECT 1",
			},
		},
	}

	code, _, _ := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusOK, code)
}

// test of auth ok/inline/plaintext/byQuery was already made in TestProfilerPayloadOnMem*

func TestAuthKOInlinePlaintextByQuery(t *testing.T) {
	cfg := db{
		Auth: &authr{
			Mode:    "INLINE",
			ByQuery: "SELECT 1 FROM AUTH WHERE USER = :user AND PASS = :password",
		},
		Macros: []macro{
			{
				Id: "M1",
				Statements: []string{
					"CREATE TABLE IF NOT EXISTS TBL (ID INT, VAL TEXT)",
					"CREATE TABLE IF NOT EXISTS AUTH (USER TEXT, PASS TEXT)",
					"DELETE FROM AUTH",
					"INSERT INTO AUTH VALUES ('myUser', 'ciao')",
				},
				Execution: execution{
					OnCreate: &TRUE,
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--mem-db", "test::env/test.yaml")(true)
	req := request{
		Credentials: &credentials{
			User:     "myUser",
			Password: "cibo",
		},
		Transaction: []requestItem{
			{
				Query: "SELECT * FROM TBL",
			},
		},
	}

	code, _, _ := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusUnauthorized, code)
}

func TestAuthOkInlinePlaintextByCreds(t *testing.T) {
	cfg := db{
		Auth: &authr{
			Mode: "INLINE",
			ByCredentials: []credentialsCfg{
				{
					User:     "myUser",
					Password: "ciao",
				},
			},
		},
		Macros: []macro{
			{
				Id: "M1",
				Statements: []string{
					"CREATE TABLE IF NOT EXISTS TBL (ID INT, VAL TEXT)",
				},
				Execution: execution{
					OnCreate: &TRUE,
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--mem-db", "test::env/test.yaml")(true)
	req := request{
		Credentials: &credentials{
			User:     "myUser",
			Password: "ciao",
		},
		Transaction: []requestItem{
			{
				Query: "SELECT * FROM TBL",
			},
		},
	}

	code, _, _ := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusOK, code)
}

func TestAuthKOInlinePlaintextByCreds(t *testing.T) {
	cfg := db{
		Auth: &authr{
			Mode: "INLINE",
			ByCredentials: []credentialsCfg{
				{
					User:     "myUser",
					Password: "ciao",
				},
			},
		},
		Macros: []macro{
			{
				Id: "M1",
				Statements: []string{
					"CREATE TABLE IF NOT EXISTS TBL (ID INT, VAL TEXT)",
				},
				Execution: execution{
					OnCreate: &TRUE,
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--mem-db", "test::env/test.yaml")(true)
	req := request{
		Credentials: &credentials{
			User:     "myUser",
			Password: "cibo",
		},
		Transaction: []requestItem{
			{
				Query: "SELECT * FROM TBL",
			},
		},
	}

	code, _, _ := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusUnauthorized, code)
}

func TestAuthOkInlineHashByCreds(t *testing.T) {
	cfg := db{
		Auth: &authr{
			Mode: "INLINE",
			ByCredentials: []credentialsCfg{
				{
					User:           "myUser",
					HashedPassword: "b133a0c0e9bee3be20163d2ad31d6248db292aa6dcb1ee087a2aa50e0fc75ae2",
				},
			},
		},
		Macros: []macro{
			{
				Id: "M1",
				Statements: []string{
					"CREATE TABLE IF NOT EXISTS TBL (ID INT, VAL TEXT)",
				},
				Execution: execution{
					OnCreate: &TRUE,
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--mem-db", "test::env/test.yaml")(true)
	req := request{
		Credentials: &credentials{
			User:     "myUser",
			Password: "ciao",
		},
		Transaction: []requestItem{
			{
				Query: "SELECT * FROM TBL",
			},
		},
	}

	code, _, _ := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusOK, code)
}

func TestAuthKOInlineHashByCreds(t *testing.T) {
	cfg := db{
		Auth: &authr{
			Mode: "INLINE",
			ByCredentials: []credentialsCfg{
				{
					User:           "myUser",
					HashedPassword: "b133a0c0e9bee3be20163d2ad31d6248db292aa6dcb1ee087a2aa50e0fc75ae2",
				},
			},
		},
		Macros: []macro{
			{
				Id: "M1",
				Statements: []string{
					"CREATE TABLE IF NOT EXISTS TBL (ID INT, VAL TEXT)",
				},
				Execution: execution{
					OnCreate: &TRUE,
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--mem-db", "test::env/test.yaml")(true)
	req := request{
		Credentials: &credentials{
			User:     "myUser",
			Password: "cibo",
		},
		Transaction: []requestItem{
			{
				Query: "SELECT * FROM TBL",
			},
		},
	}

	code, _, _ := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusUnauthorized, code)
}

func TestAuthOkHttpPlaintextByQuery(t *testing.T) {
	cfg := db{
		Auth: &authr{
			Mode:    "HTTP_BASIC",
			ByQuery: "SELECT 1 FROM AUTH WHERE USER = :user AND PASS = :password",
		},
		Macros: []macro{
			{
				Id: "M1",
				Statements: []string{
					"CREATE TABLE IF NOT EXISTS TBL (ID INT, VAL TEXT)",
					"CREATE TABLE IF NOT EXISTS AUTH (USER TEXT, PASS TEXT)",
					"DELETE FROM AUTH",
					"INSERT INTO AUTH VALUES ('myUser', 'ciao')",
				},
				Execution: execution{
					OnCreate: &TRUE,
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--mem-db", "test::env/test.yaml")(true)
	req := request{
		Transaction: []requestItem{
			{
				Query: "SELECT * FROM TBL",
			},
		},
	}

	code, _, _ := callWithAuth(t, "http://localhost:12321/test", req, "myUser", "ciao")

	require.Equal(t, http.StatusOK, code)
}

func TestAuthKOHttpPlaintextByQuery(t *testing.T) {
	cfg := db{
		Auth: &authr{
			Mode:    "HTTP_BASIC",
			ByQuery: "SELECT 1 FROM AUTH WHERE USER = :user AND PASS = :password",
		},
		Macros: []macro{
			{
				Id: "M1",
				Statements: []string{
					"CREATE TABLE IF NOT EXISTS TBL (ID INT, VAL TEXT)",
					"CREATE TABLE IF NOT EXISTS AUTH (USER TEXT, PASS TEXT)",
					"DELETE FROM AUTH",
					"INSERT INTO AUTH VALUES ('myUser', 'ciao')",
				},
				Execution: execution{
					OnCreate: &TRUE,
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--mem-db", "test::env/test.yaml")(true)
	req := request{
		Transaction: []requestItem{
			{
				Query: "SELECT * FROM TBL",
			},
		},
	}

	code, _, _ := callWithAuth(t, "http://localhost:12321/test", req, "myUser", "cibo")

	require.Equal(t, http.StatusUnauthorized, code)
}

func TestAuthOkHttpPlaintextByCreds(t *testing.T) {
	cfg := db{
		Auth: &authr{
			Mode: "HTTP_BASIC",
			ByCredentials: []credentialsCfg{
				{
					User:     "myUser",
					Password: "ciao",
				},
			},
		},
		Macros: []macro{
			{
				Id: "M1",
				Statements: []string{
					"CREATE TABLE IF NOT EXISTS TBL (ID INT, VAL TEXT)",
				},
				Execution: execution{
					OnCreate: &TRUE,
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--mem-db", "test::env/test.yaml")(true)
	req := request{
		Transaction: []requestItem{
			{
				Query: "SELECT * FROM TBL",
			},
		},
	}

	code, _, _ := callWithAuth(t, "http://localhost:12321/test", req, "myUser", "ciao")

	require.Equal(t, http.StatusOK, code)
}

func TestAuthKOHttpPlaintextByCreds(t *testing.T) {
	cfg := db{
		Auth: &authr{
			Mode: "INLINE",
			ByCredentials: []credentialsCfg{
				{
					User:     "myUser",
					Password: "ciao",
				},
			},
		},
		Macros: []macro{
			{
				Id: "M1",
				Statements: []string{
					"CREATE TABLE IF NOT EXISTS TBL (ID INT, VAL TEXT)",
				},
				Execution: execution{
					OnCreate: &TRUE,
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--mem-db", "test::env/test.yaml")(true)
	req := request{
		Transaction: []requestItem{
			{
				Query: "SELECT * FROM TBL",
			},
		},
	}

	code, _, _ := callWithAuth(t, "http://localhost:12321/test", req, "myUser", "cibo")

	require.Equal(t, http.StatusUnauthorized, code)
}

func TestAuthOkHttpHashByCreds(t *testing.T) {
	cfg := db{
		Auth: &authr{
			Mode: "INLINE",
			ByCredentials: []credentialsCfg{
				{
					User:           "myUser",
					HashedPassword: "b133a0c0e9bee3be20163d2ad31d6248db292aa6dcb1ee087a2aa50e0fc75ae2",
				},
			},
		},
		Macros: []macro{
			{
				Id: "M1",
				Statements: []string{
					"CREATE TABLE IF NOT EXISTS TBL (ID INT, VAL TEXT)",
				},
				Execution: execution{
					OnCreate: &TRUE,
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--mem-db", "test::env/test.yaml")(true)
	req := request{
		Credentials: &credentials{
			User:     "myUser",
			Password: "ciao",
		},
		Transaction: []requestItem{
			{
				Query: "SELECT * FROM TBL",
			},
		},
	}

	code, _, _ := callWithAuth(t, "http://localhost:12321/test", req, "myuser", "ciao")

	require.Equal(t, http.StatusOK, code)
}

func TestAuthKOHttpHashByCreds(t *testing.T) {
	cfg := db{
		Auth: &authr{
			Mode: "INLINE",
			ByCredentials: []credentialsCfg{
				{
					User:           "myUser",
					HashedPassword: "b133a0c0e9bee3be20163d2ad31d6248db292aa6dcb1ee087a2aa50e0fc75ae2",
				},
			},
		},
		Macros: []macro{
			{
				Id: "M1",
				Statements: []string{
					"CREATE TABLE IF NOT EXISTS TBL (ID INT, VAL TEXT)",
				},
				Execution: execution{
					OnCreate: &TRUE,
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--mem-db", "test::env/test.yaml")(true)
	req := request{
		Credentials: &credentials{
			User:     "myUser",
			Password: "cibo",
		},
		Transaction: []requestItem{
			{
				Query: "SELECT * FROM TBL",
			},
		},
	}

	code, _, _ := callWithAuth(t, "http://localhost:12321/test", req, "myuser", "cibo")

	require.Equal(t, http.StatusUnauthorized, code)
}

func TestAuthWhenBothPlaintextAndHash(t *testing.T) {
	cfg := db{
		Auth: &authr{
			Mode: "INLINE",
			ByCredentials: []credentialsCfg{
				{
					User:           "myUser",
					Password:       "cibo",                                                             // This wins
					HashedPassword: "b133a0c0e9bee3be20163d2ad31d6248db292aa6dcb1ee087a2aa50e0fc75ae2", // "ciao"
				},
			},
		},
		Macros: []macro{
			{
				Id: "M1",
				Statements: []string{
					"CREATE TABLE IF NOT EXISTS TBL (ID INT, VAL TEXT)",
				},
				Execution: execution{
					OnCreate: &TRUE,
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--mem-db", "test::env/test.yaml")(true)
	req := request{
		Credentials: &credentials{
			User:     "myUser",
			Password: "cibo",
		},
		Transaction: []requestItem{
			{
				Query: "SELECT * FROM TBL",
			},
		},
	}

	code, _, _ := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusOK, code)
}

func TestAuthNoPasswordFails(t *testing.T) {
	cfg := db{
		Auth: &authr{
			Mode: "INLINE",
			ByCredentials: []credentialsCfg{
				{
					User: "myUser",
				},
			},
		},
	}

	os.Remove("env/test.yaml")

	data, err := yaml.Marshal(cfg)
	require.NoError(t, err)

	require.NoError(t, os.WriteFile("env/test.yaml", data, 0600))
	defer os.Remove("env/test.yaml")

	cmd = exec.Command(COMMAND, "--mem-db", "test::env/test.yaml")
	require.Error(t, cmd.Run())
}

func TestAuthKOCustomErrorCode(t *testing.T) {
	cfg := db{
		Auth: &authr{
			AuthErrorCode: &custAuthError,
			Mode:          "INLINE",
			ByCredentials: []credentialsCfg{
				{
					User:     "myUser",
					Password: "ciao",
				},
			},
		},
		Macros: []macro{
			{
				Id: "M1",
				Statements: []string{
					"CREATE TABLE IF NOT EXISTS TBL (ID INT, VAL TEXT)",
				},
				Execution: execution{
					OnCreate: &TRUE,
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--mem-db", "test::env/test.yaml")(true)
	req := request{
		Credentials: &credentials{
			User:     "myUser",
			Password: "cibo",
		},
		Transaction: []requestItem{
			{
				Query: "SELECT * FROM TBL",
			},
		},
	}

	code, _, _ := call(t, "http://localhost:12321/test", req)

	require.Equal(t, custAuthError, code)
}

func TestBothValueAndBatchFail(t *testing.T) {
	cfg := db{
		Macros: []macro{
			{
				Id: "M1",
				Statements: []string{
					"CREATE TABLE IF NOT EXISTS TBL (ID INT, VAL TEXT)",
				},
				Execution: execution{
					OnCreate: &TRUE,
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--mem-db", "test::env/test.yaml")(true)
	req := request{
		Transaction: []requestItem{
			{
				Statement: "INSERT INTO TBL (ID, VAL) VALUES (:id, :val)",
				Values:    mkNamedParams(map[string]interface{}{"id": 0, "val": "zero"}),
				ValuesBatch: []map[string]json.RawMessage{
					mkNamedParams(map[string]interface{}{"id": 1, "val": "uno"}),
					mkNamedParams(map[string]interface{}{"id": 2, "val": "due"}),
				},
			},
		},
	}

	code, _, _ := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusBadRequest, code)
}

func TestCORSOk(t *testing.T) {
	cfg := db{
		CORSOrigin: "*",
	}

	defer setupTest(t, &cfg, false, "--mem-db", "test::env/test.yaml")(true)

	corsReq, _ := http.NewRequest("OPTIONS", "http://localhost:12321/test", nil)
	corsReq.Header.Add("Access-Control-Request-Method", "POST")
	corsReq.Header.Add("Origin", "http://mydomain.com")

	res, _ := http.DefaultClient.Do(corsReq)
	require.True(t, res.Header.Get("Access-Control-Allow-Origin") != "")
}

func TestCORSKO(t *testing.T) {
	defer setupTest(t, nil, false, "--mem-db", "test")(true)

	corsReq, _ := http.NewRequest("OPTIONS", "http://localhost:12321/test", nil)
	corsReq.Header.Add("Access-Control-Request-Method", "POST")
	corsReq.Header.Add("Origin", "http://mydomain.com")

	res, _ := http.DefaultClient.Do(corsReq)
	require.False(t, res.Header.Get("Access-Control-Allow-Origin") != "")
}

func TestCORSOk2(t *testing.T) {
	cfg := db{
		CORSOrigin: "http://mydomain.com",
	}

	defer setupTest(t, &cfg, false, "--mem-db", "test::env/test.yaml")(true)

	corsReq, _ := http.NewRequest("OPTIONS", "http://localhost:12321/test", nil)
	corsReq.Header.Add("Access-Control-Request-Method", "POST")
	corsReq.Header.Add("Origin", "http://mydomain.com")

	res, _ := http.DefaultClient.Do(corsReq)
	require.True(t, res.Header.Get("Access-Control-Allow-Origin") != "")
}

func TestCORSKO2(t *testing.T) {
	cfg := db{
		CORSOrigin: "http://mydomainNOT.com",
	}

	defer setupTest(t, &cfg, false, "--mem-db", "test::env/test.yaml")(true)

	corsReq, _ := http.NewRequest("OPTIONS", "http://localhost:12321/test", nil)
	corsReq.Header.Add("Access-Control-Request-Method", "POST")
	corsReq.Header.Add("Origin", "http://mydomain.com")

	res, _ := http.DefaultClient.Do(corsReq)
	require.False(t, res.Header.Get("Access-Control-Allow-Origin") != "")
}

func TestOutOfTransactionMacro(t *testing.T) {
	cfg := db{
		Macros: []macro{
			{
				Id:                 "M1",
				DisableTransaction: &TRUE,
				Statements: []string{
					"VACUUM",
				},
				Execution: execution{
					WebService: &webService{},
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--db", "env/test.db")(true)

	req := request{
		Transaction: []requestItem{},
	}

	code, res, _ := call(t, "http://localhost:12321/test/macro/M1", req)

	require.Equal(t, http.StatusOK, code, res)
}

func TestStartupBackupFailForSameDir(t *testing.T) {
	cfg := db{
		Backup: backup{
			BackupDir: "env/",
			NumFiles:  1,
		},
	}

	saveCfgToYaml(t, &cfg)

	cmd := exec.Command(COMMAND, "--db", "env/test.db")
	defer os.Remove("env/test.db")
	defer os.Remove("env/test.db-shm")
	defer os.Remove("env/test.db-wal")
	err := cmd.Run()
	require.Error(t, err)
}

func TestReturnedString(t *testing.T) {
	cfg := db{
		Macros: []macro{
			{
				Id: "M1",
				Statements: []string{
					"CREATE TABLE IF NOT EXISTS TBL (ID INT, VAL TEXT)",
				},
				Execution: execution{
					OnCreate: &TRUE,
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--mem-db", "test::env/test.yaml")(true)
	req := request{
		Transaction: []requestItem{
			{
				Statement: "INSERT INTO TBL (ID, VAL) VALUES (1, :val)",
				Values:    mkNamedParams(map[string]interface{}{"val": ciao}),
			}, {
				Query: "SELECT VAL FROM TBL WHERE ID = 1",
			},
		},
	}

	code, _, ret := call(t, "http://localhost:12321/test", req)

	require.Equal(t, http.StatusOK, code)

	// Without serde-rusqlite, it used to return "ciao" and not ciao
	require.Equal(t, ciao, ret.Results[1].ResultSet[0]["VAL"])
}

func TestReturnedBigInteger(t *testing.T) {
	cfg := db{
		Macros: []macro{
			{
				Id: "M1",
				Statements: []string{
					"CREATE TABLE IF NOT EXISTS TBL (VAL INT)",
				},
				Execution: execution{
					OnCreate: &TRUE,
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--mem-db", "test::env/test.yaml")(true)

	var test int64 = 9223372036854775807
	req := request{
		Transaction: []requestItem{
			{
				Statement: "INSERT INTO TBL VALUES(:VAL)",
				Values: mkNamedParams(map[string]interface{}{
					"VAL": test,
				}),
			},
			{
				Query: "SELECT VAL FROM TBL",
			},
		},
	}

	_, body, _ := call(t, "http://localhost:12321/test", req)

	require.True(t, strings.Contains(body, fmt.Sprintf("%d", test)))
}

func TestReturnedFloat(t *testing.T) {
	cfg := db{
		Macros: []macro{
			{
				Id: "M1",
				Statements: []string{
					"CREATE TABLE IF NOT EXISTS TBL (VAL NUMBER)",
				},
				Execution: execution{
					OnCreate: &TRUE,
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--mem-db", "test::env/test.yaml")(true)

	var test float64 = 9223372036.854775807

	req := request{
		Transaction: []requestItem{
			{
				Statement: "INSERT INTO TBL VALUES(:VAL)",
				Values: mkNamedParams(map[string]interface{}{
					"VAL": test,
				}),
			},
			{
				Query: "SELECT VAL FROM TBL",
			},
		},
	}

	_, body, _ := call(t, "http://localhost:12321/test", req)

	require.True(t, strings.Contains(body, fmt.Sprintf("%f", test)))
}

func TestReturnedBool(t *testing.T) {
	cfg := db{
		Macros: []macro{
			{
				Id: "M1",
				Statements: []string{
					"CREATE TABLE IF NOT EXISTS TBL (VAL BOOL)",
				},
				Execution: execution{
					OnCreate: &TRUE,
				},
			},
		},
	}

	defer setupTest(t, &cfg, false, "--mem-db", "test::env/test.yaml")(true)

	req := request{
		Transaction: []requestItem{
			{
				Statement: "INSERT INTO TBL VALUES(:VAL)",
				Values: mkNamedParams(map[string]interface{}{
					"VAL": true,
				}),
			},
			{
				Query: "SELECT VAL FROM TBL",
			},
		},
	}

	_, _, ret := call(t, "http://localhost:12321/test", req)

	require.Equal(t, "true", ret.Results[1].ResultSet[0]["VAL"].(string))
}
