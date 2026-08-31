#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct ApiDefinition ApiDefinition;

typedef struct YcallrClientWrapper YcallrClientWrapper;

typedef struct YcallrCommand YcallrCommand;

typedef struct YcallrResponse YcallrResponse;

typedef struct YcallrApi {
  char *name;
  char *version;
  char *description;
  char *base_url;
  struct ApiDefinition *_inner;
} YcallrApi;

const char *ycallr_get_last_error(void);

void ycallr_error_free(char *err);

struct YcallrApi *ycallr_parse_yaml(const char *yaml);

/**
 * Load a compiled profile from `~/.config/ycallr/apis/<name>.pb`.
 */
struct YcallrApi *ycallr_load_installed(const char *name);

/**
 * Decode a compiled protobuf profile from memory (e.g. embedded or custom storage).
 */
struct YcallrApi *ycallr_parse_proto(const uint8_t *data, uintptr_t len);

/**
 * Compile `~/.config/ycallr/apis/<name>.yaml` to `<name>.pb`. Returns 0 on success.
 */
int32_t ycallr_install(const char *name);

/**
 * Install from a YAML file path (copies into apis dir when needed). Returns 0 on success.
 */
int32_t ycallr_install_yaml_file(const char *path);

void ycallr_free_api(struct YcallrApi *api);

/**
 * Override base URL at runtime (e.g. point profile at a mock server). Returns 0 on success, -1 on error.
 */
int32_t ycallr_set_base_url(struct YcallrApi *api,
                            const char *url);

const char *ycallr_get_name(const struct YcallrApi *api);

const char *ycallr_get_version(const struct YcallrApi *api);

const char *ycallr_get_base_url(const struct YcallrApi *api);

const char *ycallr_get_description(const struct YcallrApi *api);

/**
 * Declared profile env vars as JSON: `[{"name":"TOKEN","required":true}]`
 */
char *ycallr_get_env_json(const struct YcallrApi *api);

/**
 * Returns a JSON array of command names at the top level: `["repos","users"]`
 */
char *ycallr_list_commands(const struct YcallrApi *api);

/**
 * Returns JSON array of installed profile names: `["github","demo"]`.
 */
char *ycallr_list_installed(void);

/**
 * After `ycallr_install` / `ycallr_install_yaml_file`: `{"name":"...","pb_path":"..."}`.
 */
char *ycallr_get_last_install_result(void);

/**
 * Returns filesystem path to `~/.config/ycallr/apis/<name>.pb`.
 */
char *ycallr_compiled_profile_path(const char *name);

/**
 * Returns JSON array of subcommand names for `path` (use empty string for top level).
 */
char *ycallr_list_subcommands(const struct YcallrApi *api, const char *path);

/**
 * Returns JSON array of missing required parameter names before a call.
 */
char *ycallr_missing_params_json(const struct YcallrApi *api,
                                 const char *command_path,
                                 const char *params_json);

/**
 * Builds implicit JSON body for POST/PUT/PATCH when YAML has no body; NULL if none.
 */
char *ycallr_build_implicit_body_json(const struct YcallrApi *api,
                                      const char *command_path,
                                      const char *params_json);

struct YcallrCommand *ycallr_get_command(const struct YcallrApi *api, const char *path);

void ycallr_free_command(struct YcallrCommand *cmd);

char *ycallr_command_get_endpoint(const struct YcallrCommand *cmd);

char *ycallr_command_get_method(const struct YcallrCommand *cmd);

char *ycallr_command_get_description(const struct YcallrCommand *cmd);

char *ycallr_command_get_auth(const struct YcallrCommand *cmd);

char *ycallr_command_get_headers_json(const struct YcallrCommand *cmd);

char *ycallr_command_get_params_json(const struct YcallrCommand *cmd);

bool ycallr_command_is_leaf(const struct YcallrCommand *cmd);

bool ycallr_command_is_branch(const struct YcallrCommand *cmd);

bool ycallr_command_has_body(const struct YcallrCommand *cmd);

/**
 * Returns body kind: `json`, `form`, `raw`, or `multipart`. Caller frees with `ycallr_string_free`.
 */
char *ycallr_command_get_body_kind(const struct YcallrCommand *cmd);

/**
 * Returns JSON array of path parameter names from the endpoint template.
 */
char *ycallr_command_get_path_params_json(const struct YcallrCommand *cmd);

/**
 * Create a client. env_mode: 0=Auto, 1=Manual. envs_json: `{"KEY":"val"}` or NULL.
 */
struct YcallrClientWrapper *ycallr_client_new(const struct YcallrApi *api,
                                              uint8_t env_mode,
                                              const char *envs_json);

/**
 * Create a client with auth. auth_type: "bearer"|"api_key"|"http_basic"|"http_custom".
 * auth_data_json varies by type:
 *   bearer:       {"token":"xxx"}
 *   api_key:      {"key":"xxx","name":"X-API-Key","in":"header"|"query"|"cookie"}
 *   http_basic:   {"username":"u","password":"p"}
 *   http_custom:  {"prefix":"xxx","token":"yyy"}
 */
struct YcallrClientWrapper *ycallr_client_new_with_auth(const struct YcallrApi *api,
                                                        const char *auth_type,
                                                        const char *auth_data_json,
                                                        uint8_t env_mode,
                                                        const char *envs_json);

void ycallr_client_free(struct YcallrClientWrapper *client);

struct YcallrResponse *ycallr_call(const struct YcallrClientWrapper *client,
                                   const char *command,
                                   const char *params_json,
                                   const char *body_json);

void ycallr_free_response(struct YcallrResponse *resp);

uint16_t ycallr_response_get_status(const struct YcallrResponse *resp);

/**
 * Returns headers as JSON object: `{"content-type":"application/json",...}`.
 * Caller must free with ycallr_string_free().
 */
char *ycallr_response_get_headers_json(const struct YcallrResponse *resp);

/**
 * Returns body as JSON string. Caller must free with ycallr_string_free().
 */
char *ycallr_response_get_body_json(const struct YcallrResponse *resp);

/**
 * Returns response message (if configured in YAML) or NULL.
 * Caller must free with ycallr_string_free() if non-null.
 */
char *ycallr_response_get_message(const struct YcallrResponse *resp);

/**
 * Free a string returned by any ycallr_response_get_* or ycallr_list_commands.
 */
void ycallr_string_free(char *s);
