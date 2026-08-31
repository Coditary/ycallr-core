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
 * Returns a JSON array of command names at the top level: `["repos","users"]`
 */
char *ycallr_list_commands(const struct YcallrApi *api);

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
