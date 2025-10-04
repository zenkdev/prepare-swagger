# prepare-swagger

## usage

```bash
prepare-swagger path_to_config.yaml path_to_schema.yaml
```

## schema

```yaml
url: https://petstore.swagger.io/v2/swagger.json
request:
  # optional headers
  headers:
    x-sc-lb-hint: default
paths:
  # key: search regex, value: replacement
  ^/pet(/.*): /my_pet$1
  __remove:
    - path_to_remove
definitions:
  # definitions to add
  AddDto:
    properties:
      id:
        type: string
    required:
      - '*' # all properties
  # definitions to override
  ModifyDto:
    properties:
      id:
        type: string
      # properties to remove
      __remove:
        - key
    # specific properties
    required:
      - id
  # definitions to remove
  __remove:
    - RemoveDto
```
