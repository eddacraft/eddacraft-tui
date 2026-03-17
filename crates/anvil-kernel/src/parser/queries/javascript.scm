;; Functions
(function_declaration
  name: (identifier) @name) @function

(lexical_declaration
  (variable_declarator
    name: (identifier) @name
    value: (arrow_function))) @function

;; Classes
(class_declaration
  name: (identifier) @name) @class

;; Exports (CJS)
(expression_statement
  (assignment_expression
    left: (member_expression
      object: (identifier) @_obj
      property: (property_identifier) @_prop)
    (#eq? @_obj "module")
    (#eq? @_prop "exports"))) @export

;; Exports (ESM)
(export_statement) @export

;; Imports (ESM)
(import_statement
  source: (string) @source) @import

;; Imports (CJS)
(lexical_declaration
  (variable_declarator
    value: (call_expression
      function: (identifier) @_fn
      arguments: (arguments (string) @source)
      (#eq? @_fn "require")))) @import
