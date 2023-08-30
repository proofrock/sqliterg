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
	"io/ioutil"
	"net/http"
	"os"
	"os/exec"
	"testing"
	"time"

	"github.com/stretchr/testify/require"
)

const COMMAND = "../target/debug/sqliterg"

func TestMain(m *testing.M) {
	println("Go...")
	exitCode := m.Run()
	println("...finished")
	os.Exit(exitCode)
}

var cmd *exec.Cmd

func setupTest(t *testing.T, argv ...string) func() {
	cmd = exec.Command(COMMAND, argv...)
	err := cmd.Start()
	require.NoError(t, err)
	time.Sleep(1 * time.Second)

	return func() {
		cmd.Process.Kill()
		os.Remove("env/test.db")
		os.Remove("env/test.yaml")
	}
}

func TestErrorNoArgs(t *testing.T) {
	cmd := exec.Command(COMMAND)
	err := cmd.Run()
	require.Error(t, err)
}

func TestFileServer(t *testing.T) {
	defer setupTest(t, "--serve-dir", ".")()

	resp, err := http.Get("http://localhost:12321/env/test.1")
	require.NoError(t, err)

	require.Equal(t, http.StatusOK, resp.StatusCode)
	bs, err := ioutil.ReadAll(resp.Body)
	require.NoError(t, err)
	require.Equal(t, "1", string(bs))
}

func TestMemDbEmpty(t *testing.T) {
	defer setupTest(t, "--mem-db", "test")()

	req := []byte("{\"transaction\":[{\"query\":\"SELECT 1\"}]}")
	post, err := http.NewRequest("POST", "http://localhost:12321/test/exec", bytes.NewBuffer(req))
	require.NoError(t, err)
	post.Header.Add("Content-Type", "application/json")

	resp, err := http.DefaultClient.Do(post)
	require.NoError(t, err)

	require.Equal(t, http.StatusOK, resp.StatusCode)
	bs, err := ioutil.ReadAll(resp.Body)
	require.NoError(t, err)
	var obj response
	json.Unmarshal(bs, &obj)
	require.Equal(t, 1, len(obj.Results))
	require.True(t, obj.Results[0].Success)
	require.Equal(t, 1, len(obj.Results[0].ResultSet))
	require.Equal(t, 1, len(obj.Results[0].ResultSet[0]))
	require.Equal(t, 1, int(obj.Results[0].ResultSet[0]["1"].(float64)))
}

func TestFileDbEmpty(t *testing.T) {
	defer setupTest(t, "--db", "env/test.db")()

	require.FileExists(t, "env/test.db")

	req := []byte("{\"transaction\":[{\"query\":\"SELECT 1\"}]}")
	post, err := http.NewRequest("POST", "http://localhost:12321/test/exec", bytes.NewBuffer(req))
	require.NoError(t, err)
	post.Header.Add("Content-Type", "application/json")

	resp, err := http.DefaultClient.Do(post)
	require.NoError(t, err)

	require.Equal(t, http.StatusOK, resp.StatusCode)
	bs, err := ioutil.ReadAll(resp.Body)
	require.NoError(t, err)
	var obj response
	json.Unmarshal(bs, &obj)
	require.Equal(t, 1, len(obj.Results))
	require.True(t, obj.Results[0].Success)
	require.Equal(t, 1, len(obj.Results[0].ResultSet))
	require.Equal(t, 1, len(obj.Results[0].ResultSet[0]))
	require.Equal(t, 1, int(obj.Results[0].ResultSet[0]["1"].(float64)))
}
