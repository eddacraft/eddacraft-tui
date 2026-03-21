;; Functions (named + arrow)
(function_declaration
  name: (identifier) @name) @function

(lexical_declaration
  (variable_declarator
    name: (identifier) @name
    value: (arrow_function))) @function

;; Classes
(class_declaration
  name: (type_identifier) @name) @class

;; Exports
(export_statement) @export

;; Imports
(import_statement
  source: (string) @source) @import
