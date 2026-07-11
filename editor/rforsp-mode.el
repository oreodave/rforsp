;;; rforsp-mode.el --- Major mode for the rForsp language  -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Aryadev Chavali
;; Author: Aryadev Chavali

;; This program is free software: you can redistribute it and/or
;; modify it under the terms of the MIT License.


;;; Commentary:

;; A major mode providing syntax highlighting and basic indentation for rForsp
;; source files (.rfp).

;; rForsp is a hybrid language of Forth and Lisp based on xorvoid's Forsp.  See
;; https://github.com/oreodave/rforsp.

;;; Code:

(require 'imenu)

(defgroup rforsp nil
  "Major mode for rForsp."
  :group 'languages)

(defcustom rforsp-indent-offset 2
  "Number of spaces per nesting level."
  :type 'integer)

(defcustom rforsp-builtins
  (list "quote" "if")
  "Builtin words to highlight."
  :type '(repeat string))

(defcustom rforsp-keywords
  (list
   "copy" "length" "vmake" "vpush" "vpop" "vswap" "vget" "vset"
   "push" "pop" "cons" "car" "cdr" "eq" "cswap"
   "tag" "read" "print" "stack" "env"
   "+" "-" "*" "/" "&" "|" "nand" "<<" ">>"
   "rec")
  "Keyword list."
  :type '(repeat string))

(defcustom rforsp-formatter-command nil
  "Future formatter command."
  :type '(choice (const nil) string))

;;; Syntax table

(defvar rforsp-mode-syntax-table
  (let ((st (make-syntax-table)))

    ;; Lisp-style comments.
    (modify-syntax-entry ?\; "<" st)
    (modify-syntax-entry ?\n ">" st)

    ;; Strings (stub; multiline supported).
    (modify-syntax-entry ?\" "\"" st)

    ;; Parentheses.
    (modify-syntax-entry ?\( "()" st)
    (modify-syntax-entry ?\) ")(" st)

    ;; Brackets.
    (modify-syntax-entry ?\[ "(]" st)
    (modify-syntax-entry ?\] ")[" st)

    ;; Treat these as punctuation so we can fontify them.
    (modify-syntax-entry ?' "." st)
    (modify-syntax-entry ?^ "." st)
    (modify-syntax-entry ?$ "." st)

    ;; Symbol constituents
    (modify-syntax-entry ?- "_" st)
    (modify-syntax-entry ?+ "_" st)
    (modify-syntax-entry ?* "_" st)
    (modify-syntax-entry ?/ "_" st)
    (modify-syntax-entry ?& "_" st)
    (modify-syntax-entry ?| "_" st)
    (modify-syntax-entry ?< "_" st)
    (modify-syntax-entry ?> "_" st)

    st))

;;; Font lock

(defconst rforsp--symbol
  "[^][() \t\n;\"'^$]+")

(defconst rforsp-font-lock-keywords
  `(
    ;; Reader operators.
    (,(rx (group "$") (group (regexp rforsp--symbol)))
     (1 font-lock-builtin-face)
     (2 font-lock-variable-name-face))

    (,(rx (group "^") (regexp rforsp--symbol))
     (1 font-lock-builtin-face))

    (,(rx (group "'") (regexp rforsp--symbol))
     (1 font-lock-builtin-face))

    (,(concat "\\_<" (regexp-opt rforsp-builtins t) "\\_>")
     . font-lock-builtin-face)

    (,(concat "\\_<" (regexp-opt rforsp-keywords t) "\\_>")
     . font-lock-keyword-face)

    ;; Future: integers.
    ;; ("\\_<[0-9]+\\_>" . font-lock-constant-face)

    ;; Future: floats.
    ;; ("\\_<[0-9]+\\.[0-9]+\\_>" . font-lock-constant-face)

    ;; Future: hex/bin/octal.
    ;; Future: character literals.
    ))

;;; Indentation

(defun rforsp--nesting-depth (limit)
  "Return delimiter nesting depth before LIMIT."
  (save-excursion
    (goto-char (point-min))
    (cl-loop
     with depth = 0
     while (< (point) limit)
     for state = (syntax-ppss)
     if (not (or (nth 3 state)
                 (nth 4 state)))
     do (pcase (char-after)
          ((or ?\[ ?\() (setq depth (1+ depth)))
          ((or ?\] ?\))
           (setq depth (max 0 (1- depth)))))
     ;; On every loop
     do (forward-char 1)
     finally (return depth))))

(defun rforsp-indent-line ()
  "Indent current line."
  (interactive)
  (let* ((pos (- (point-max) (point)))
         (bol (line-beginning-position))
         (depth (rforsp--nesting-depth bol)))
    (save-excursion
      (back-to-indentation)
      (when (looking-at "[])]")
        (setq depth (max 0 (1- depth))))
      (indent-line-to (* depth rforsp-indent-offset)))
    (when (> (- (point-max) pos) (point))
      (goto-char (- (point-max) pos)))))

;;; Imenu

(defconst rforsp-imenu-generic-expression
  `(("Definitions"
     ,(rx (seq
           "]" (one-or-more whitespace)
           (zero-or-one
            (seq "rec" (one-or-more whitespace)))
           "$" (group (regexp rforsp--symbol))))
     1)))

;;; Defun navigation

(defun rforsp-beginning-of-defun (&optional arg)
  (interactive "^p")
  (setq arg (or arg 1))
  (if (< arg 0)
      (rforsp-end-of-defun (- arg))
    (dotimes (_ arg)
      (re-search-backward
       "^[ \t]*\\["
       nil t))))

(defun rforsp-end-of-defun (&optional arg)
  (interactive "^p")
  (setq arg (or arg 1))
  (if (< arg 0)
      (rforsp-beginning-of-defun (- arg))
    (dotimes (_ arg)
      (re-search-forward
       "]\\s-*\\$"
       nil t))))

;;; Smart parens integration
(with-eval-after-load "smartparens"
  (defun rforsp--smartparens-config ()
    (setq-local
     sp-local-pairs
     '((:open "\\\"" :close "\\\"" :actions (insert wrap autoskip navigate))
       (:open "\"" :close "\"" :actions (wrap insert autoskip navigate) :unless
        (sp-point-before-word-p sp-point-after-word-p sp-point-before-same-p)
        :post-handlers nil)
       (:open "(" :close ")" :actions (insert wrap autoskip navigate))
       (:open "[" :close "]" :actions (insert wrap autoskip navigate))))))

;;;###autoload
(add-to-list 'auto-mode-alist '("\\.rfp\\'" . rforsp-mode))

;;;###autoload
(define-derived-mode rforsp-mode prog-mode "rForsp"
  "Major mode for rForsp."

  :syntax-table rforsp-mode-syntax-table

  (setq-local evil-shift-width rforsp-indent-offset)

  (setq-local font-lock-defaults
              '(rforsp-font-lock-keywords))

  (setq-local indent-line-function
              #'rforsp-indent-line)

  (setq-local comment-start ";")
  (setq-local comment-end "")

  (setq-local imenu-generic-expression
              rforsp-imenu-generic-expression)

  (setq-local beginning-of-defun-function
              #'rforsp-beginning-of-defun)

  (setq-local end-of-defun-function
              #'rforsp-end-of-defun)

  (setq-local outline-regexp "^\\s-*\\[")

  (with-eval-after-load "smartparens"
    (rforsp--smartparens-config)))

(provide 'rforsp-mode)

;;; rforsp-mode.el ends here
