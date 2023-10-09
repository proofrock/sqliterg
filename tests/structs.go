/*
  Copyright (c) 2022-, Germano Rizzo <oss /AT/ germanorizzo /DOT/ it>

  Permission to use, copy, modify, and/or distribute this software for any
  purpose with or without fee is hereby granted, provided that the above
  copyright notice and this permission notice appear in all copies.

  THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
  WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
  MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
  ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
  WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
  ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
  OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
*/

package main

// These are for parsing the config file (from YAML)

type credentialsCfg struct {
	User           string `yaml:"user,omitempty"`
	Password       string `yaml:"password,omitempty"`
	HashedPassword string `yaml:"hashedPassword,omitempty"`
}

type authr struct {
	AuthErrorCode   *int             `yaml:"authErrorCode,omitempty"`
	Mode            string           `yaml:"mode,omitempty"` // 'INLINE' or 'HTTP_BASIC'
	CustomErrorCode *int             `yaml:"customErrorCode,omitempty"`
	ByQuery         string           `yaml:"byQuery,omitempty"`
	ByCredentials   []credentialsCfg `yaml:"byCredentials,omitempty"`
}

type storedStatement struct {
	Id  string `yaml:"id,omitempty"`
	Sql string `yaml:"sql,omitempty"`
}

type webService struct {
	AuthErrorCode   *int    `yaml:"authErrorCode,omitempty"`
	AuthToken       *string `yaml:"authToken,omitempty"`
	HashedAuthToken *string `yaml:"hashedAuthToken,omitempty"`
}

type execution struct {
	OnCreate   *bool       `yaml:"onCreate,omitempty"`
	OnStartup  *bool       `yaml:"onStartup,omitempty"`
	Period     *uint       `yaml:"period,omitempty"`
	WebService *webService `yaml:"webService,omitempty"`
}

type macro struct {
	Id                 string    `yaml:"id,omitempty"`
	DisableTransaction *bool     `yaml:"disableTransaction,omitempty"`
	Statements         []string  `yaml:"statements,omitempty"`
	Execution          execution `yaml:"execution,omitempty"`
}

type backup struct {
	BackupDir string    `yaml:"backupDir,omitempty"`
	NumFiles  uint      `yaml:"numFiles,omitempty"`
	Execution execution `yaml:"execution,omitempty"`
}

type db struct {
	Auth                    *authr            `yaml:"auth,omitempty"`
	ReadOnly                bool              `yaml:"readOnly,omitempty"`
	CORSOrigin              string            `yaml:"corsOrigin,omitempty"`
	UseOnlyStoredStatements bool              `yaml:"useOnlyStoredStatements,omitempty"`
	JournalMode             string            `yaml:"journalMode,omitempty"`
	StoredStatement         []storedStatement `yaml:"storedStatements,omitempty"`
	Macros                  []macro           `yaml:"macros,omitempty"`
	Backup                  backup            `yaml:"backup,omitempty"`
}

// These are for parsing the request (from JSON)

type credentials struct {
	User     string `json:"user,omitempty"`
	Password string `json:"password,omitempty"`
}

type requestItem struct {
	Query       string      `json:"query,omitempty"`
	Statement   string      `json:"statement,omitempty"`
	NoFail      bool        `json:"noFail,omitempty"`
	Values      interface{} `json:"values,omitempty"`
	ValuesBatch interface{} `json:"valuesBatch,omitempty"`
}

type request struct {
	Credentials *credentials  `json:"credentials,omitempty"`
	Transaction []requestItem `json:"transaction,omitempty"`
}

// These are for generating the response

type responseItem struct {
	Success          bool                     `json:"success"`
	RowsUpdated      *int                     `json:"rowsUpdated,omitempty"`
	RowsUpdatedBatch []int                    `json:"rowsUpdatedBatch,omitempty"`
	ResultSet        []map[string]interface{} `json:"resultSet"`
	Error            string                   `json:"error,omitempty"`
}

type response struct {
	Results []responseItem `json:"results"`
}
