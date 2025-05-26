# Version history

## 0.1.3 (26-05-2025)

### Features
- add `TryFrom` implementation for `AppConfigs` and simplify config handling

### Refactors
- remove the attribute, `async` is in the new version of `Rust` from the `std` library

## 0.1.2 (23-05-2025)

### Features
- move to a separate file and optimized the algorithm of matching and parsing of `Allowed Origin` for `CORS`

### Chores
- improve the code: better names for variables and functions, change a better methods for error inspecting, etc.
- improve performance

## 0.1.1 (07-05-2025)

### Features
- implement custom predicate matcher for dynamic `origins`

### Chores
- change the allowed list of origins for `CORS` with support for wildcard sources (e.g. `*.domain.com`)

## 0.1.0 (24-04-2025)

### Initial Release
- the basic version of the microservice that sends a message to @mail from the `Let's start` form is ready