# File Opening Capabilities Specification

## Purpose

Define runtime file-opening strategies while preserving web security boundaries and enabling desktop-native local opening.

## Requirements

### Requirement: Web File Opening Strategies

The web runtime MUST keep browser local upload available. Server browsing MUST remain root-scoped and config-gated by the server file root.

#### Scenario: Web Home keeps browser upload

- GIVEN Logmancer is running as a normal web deployment
- WHEN the user opens Home
- THEN Home MUST present browser local upload
- AND selecting a browser file MUST use the existing upload flow

#### Scenario: Web server browser remains disabled without root

- GIVEN Logmancer is running as a normal web deployment without a configured server file root
- WHEN the user opens Home
- THEN server browsing MUST NOT allow selecting server files
- AND Home MAY explain that server browsing requires configuration

#### Scenario: Web server browser remains root-scoped

- GIVEN Logmancer is running as a normal web deployment with a configured server file root
- WHEN the user browses or opens a server file
- THEN all browsed paths MUST be constrained to that root
- AND path traversal outside the root MUST be rejected

### Requirement: Desktop Home File Opening Strategies

The desktop runtime MUST NOT present browser-style upload on Home. It MUST provide local opening without requiring `server_file_root`.

#### Scenario: Desktop Home hides browser upload

- GIVEN Logmancer is running in the desktop shell
- WHEN the user opens Home
- THEN Home MUST NOT present browser local upload
- AND Home MUST present a desktop-native open-file action

#### Scenario: Desktop opens local file without server root

- GIVEN Logmancer is running in the desktop shell without `server_file_root`
- WHEN the user chooses a local text log through the desktop-native open-file action
- THEN the file MUST open successfully
- AND the result MUST navigate to `/log/{file_id}`

### Requirement: Desktop Direct Open Registry Consistency

Desktop direct open MUST route files to the same registry/session used by the embedded desktop web server so `/log/{file_id}` resolves.

#### Scenario: Desktop opened file is readable by embedded server

- GIVEN a desktop-native open action returns a `file_id`
- WHEN the desktop webview navigates to `/log/{file_id}`
- THEN the embedded web server MUST find that file in its active registry/session
- AND the log view MUST load without a missing-file error

### Requirement: Arbitrary Path Exposure Boundary

Desktop direct open MUST NOT create a generally exposed arbitrary path endpoint usable by normal web deployments.

#### Scenario: Normal web cannot open arbitrary path

- GIVEN Logmancer is running as a normal web deployment
- WHEN a client attempts to open an arbitrary absolute filesystem path through HTTP
- THEN the request MUST NOT be supported
- AND no file outside the configured server-browser root MUST be opened

#### Scenario: Desktop native open is not a general web route

- GIVEN Logmancer is running in the desktop shell
- WHEN local file opening is invoked
- THEN arbitrary path access MUST be constrained to the desktop-native capability boundary
- AND normal web deployments MUST NOT gain that capability

### Requirement: Desktop Drag And Drop Slice

Desktop local drag/drop MUST use the desktop-native file-opening capability and MUST NOT restore browser-style upload UI on Desktop Home.

#### Scenario: Desktop drag/drop uses native boundary

- GIVEN Logmancer is running in the desktop shell
- WHEN the user drops a local file path into the desktop shell
- THEN it MUST open through the desktop-native file-opening capability
- AND it MUST preserve the registry/session consistency required for `/log/{file_id}`
- AND the desktop window MUST navigate to `/log/{file_id}` after a successful drop open
