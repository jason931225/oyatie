pub(super) fn query_operation_document() -> String {
    r#"openapi: 3.2.0
info:
  title: Oyatie Query API
  version: 1.0.0
paths:
  /v1/capability-queries:
    query:
      operationId: queryCapability
      responses:
        '200':
          description: Query completed.
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/QueryResponse'
components:
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
  schemas:
    QueryResponse:
      type: object
      required:
        - data
      properties:
        data:
          type: string
          x-oyatie-rust-type: String
          x-oyatie-data-class: INTERNAL_ONLY
"#
    .into()
}

pub(super) fn copy_operation_document() -> String {
    r#"openapi: 3.2.0
info:
  title: Oyatie Copy API
  version: 1.0.0
paths:
  /v1/resources/{resource_id}:
    additionalOperations:
      COPY:
        operationId: copyResource
        security:
          - bearerAuth: []
        parameters:
          - name: resource_id
            in: path
            required: true
            schema:
              type: string
            x-oyatie-data-class: INTERNAL_ONLY
          - name: X-Request-Id
            in: header
            required: true
            schema:
              type: string
              minLength: 1
            x-oyatie-rust-type: String
            x-oyatie-data-class: INTERNAL_ONLY
          - name: X-Tenant-Id
            in: header
            required: true
            schema:
              type: string
              minLength: 1
            x-oyatie-rust-type: String
            x-oyatie-data-class: INTERNAL_ONLY
          - name: Idempotency-Key
            in: header
            required: true
            schema:
              type: string
              minLength: 1
            x-oyatie-rust-type: String
            x-oyatie-data-class: INTERNAL_ONLY
        responses:
          '200':
            description: Copy completed.
            content:
              application/json:
                schema:
                  $ref: '#/components/schemas/CopyResponse'
components:
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
  schemas:
    CopyResponse:
      type: object
      required:
        - data
      properties:
        data:
          type: string
          x-oyatie-rust-type: String
          x-oyatie-data-class: INTERNAL_ONLY
"#
    .into()
}
