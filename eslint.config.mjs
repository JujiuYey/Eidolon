import defineEslintConfig from '@antfu/eslint-config';

export default defineEslintConfig(
  {
    ignores: ['node_modules/**', 'types/components.d.ts', 'types/auto-imports.d.ts', 'public/**'],
    stylistic: {
      indent: 2,
      jsx: true,
      quotes: 'single',
      semi: true,
      overrides: {
        'camelcase': 'off',
        'style/line-comment-position': [
          'error',
          {
            position: 'above',
          },
        ],
        'style/jsx-self-closing-comp': [
          'error',
          {
            component: true,
            html: true,
          },
        ],
        'style/jsx-pascal-case': 'error',
        'style/jsx-newline': [
          'error',
          {
            prevent: true,
            allowMultilines: true,
          },
        ],
        'style/jsx-props-no-multi-spaces': 'error',
        'style/jsx-sort-props': [
          'error',
          {
            callbacksLast: true,
            shorthandFirst: true,
            shorthandLast: false,
            multiline: 'ignore',
            ignoreCase: true,
            noSortAlphabetically: false,
            reservedFirst: true,
          },
        ],
        'style/newline-per-chained-call': 'off',
        'style/object-property-newline': 'off',
        'style/function-call-spacing': [
          'error',
          'never',
        ],
        'style/function-call-argument-newline': [
          'error',
          'consistent',
        ],
        'style/arrow-parens': [
          'error',
          'as-needed',
        ],
        'style/brace-style': [
          'error',
          '1tbs',
          {
            allowSingleLine: false,
          },
        ],
        'style/jsx-quotes': [
          'error',
          'prefer-single',
        ],
      },
    },
    vue: {
      overrides: {
        'vue/prop-name-casing': 'off',
        'vue/custom-event-name-casing': 'off',
        'vue/attributes-order': 'error',
        'vue/attribute-hyphenation': [
          'error',
          'always',
        ],
        'vue/order-in-components': 'error',
        'vue/this-in-template': [
          'error',
          'never',
        ],
        'vue/block-lang': [
          'error',
          {
            script: {
              lang: ['ts'],
              allowNoLang: false,
            },
          },
        ],
        'vue/component-name-in-template-casing': [
          'error',
          'PascalCase',
          {
            registeredComponentsOnly: false,
          },
        ],
        'vue/html-comment-content-newline': [
          'error',
          {
            singleline: 'never',
            multiline: 'always',
          },
        ],
        'vue/html-comment-indent': [
          'error',
          2,
        ],
        'vue/component-definition-name-casing': 'error',
        'vue/next-tick-style': [
          'error',
          'promise',
        ],
        'vue/no-duplicate-attr-inheritance': 'off',
        'vue/no-multiple-objects-in-class': 'error',
        'vue/no-multiple-template-root': 'off',
        'vue/no-potential-component-option-typo': [
          'error',
          {
            presets: ['vue', 'vue-router'],
            custom: [],
          },
        ],
        'vue/no-required-prop-with-default': [
          'error',
          {
            autofix: true,
          },
        ],
        'vue/no-static-inline-styles': 'off',
        'vue/no-this-in-before-route-enter': 'error',
        'vue/no-unused-properties': [
          'error',
          {
            groups: ['setup'],
          },
        ],
        'vue/no-useless-mustaches': [
          'error',
          {
            ignoreIncludesComment: false,
            ignoreStringEscape: true,
          },
        ],
        'vue/no-v-text': 'error',
        'vue/prefer-prop-type-boolean-first': 'error',
        'vue/prefer-true-attribute-shorthand': 'off',
        'vue/v-for-delimiter-style': [
          'error',
          'of',
        ],
        'vue/html-self-closing': [
          'error',
          {
            html: {
              void: 'always',
              normal: 'always',
              component: 'always',
            },
            math: 'always',
            svg: 'always',
          },
        ],
        'vue/html-end-tags': 'error',
        'vue/html-closing-bracket-newline': 'error',
        'vue/html-closing-bracket-spacing': [
          'error',
          {
            startTag: 'never',
            endTag: 'never',
            selfClosingTag: 'always',
          },
        ],
        'vue/first-attribute-linebreak': [
          'error',
          {
            singleline: 'beside',
            multiline: 'below',
          },
        ],
        'vue/multiline-html-element-content-newline': 'error',
        'vue/mustache-interpolation-spacing': [
          'error',
          'always',
        ],
        'vue/no-lone-template': 'error',
        'vue/no-child-content': 'error',
        'vue/no-multi-spaces': [
          'error',
          {
            ignoreProperties: false,
          },
        ],
        'vue/no-multiple-slot-args': 'error',
        'vue/no-spaces-around-equal-signs-in-attribute': 'error',
        'vue/no-template-shadow': 'error',
        'vue/require-explicit-emits': 'error',
        'vue/v-bind-style': [
          'error',
          'shorthand',
          {
            sameNameShorthand: 'never',
          },
        ],
        'vue/v-on-event-hyphenation': [
          'error',
          'always',
          {
            autofix: true,
            ignore: [],
          },
        ],
        'vue/v-on-style': [
          'error',
          'shorthand',
        ],
        'vue/v-slot-style': [
          'error',
          {
            atComponent: 'shorthand',
            default: 'shorthand',
            named: 'shorthand',
          },
        ],
        'vue/camelcase': [
          'error',
          {
            properties: 'always',
            ignoreDestructuring: false,
            ignoreImports: false,
            ignoreGlobals: false,
            allow: [],
          },
        ],
        'vue/func-call-spacing': [
          'error',
          'never',
        ],
        'vue/no-extra-parens': 'error',
        'vue/key-spacing': [
          'error',
          {
            beforeColon: false,
            afterColon: true,
          },
        ],
      },
    },
    javascript: {
      overrides: {
        'prefer-object-spread': 'error',
        'func-style': [
          'error',
          'declaration',
          {
            allowArrowFunctions: true,
          },
        ],
        'object-shorthand': 'error',
        'camelcase': [
          'error',
          {
            properties: 'always',
            ignoreDestructuring: false,
            ignoreImports: false,
            ignoreGlobals: false,
            allow: [],
          },
        ],
        'no-console': 'off',
        'curly': [
          'error',
          'all',
        ],
        'eqeqeq': 'off',
        'no-useless-call': 'off',
        'prefer-promise-reject-errors': 'off',
        'no-template-curly-in-string': 'off',
        'sort-imports': 'off',
      },
    },
    typescript: {
      overrides: {
        'ts/no-unused-expressions': 'off',
        'ts/no-unused-vars': 'off',
        'ts/array-type': [
          'error',
          {
            default: 'array-simple',
            readonly: 'array-simple',
          },
        ],
        'ts/consistent-type-assertions': 'off',
        'ts/no-use-before-define': 'off',
        'ts/ban-ts-comment': 'off',
        'ts/ban-types': 'off',
        'ts/prefer-literal-enum-member': 'off',
        'ts/no-empty-object-type': 'off',
      },
    },
    jsx: true,
    regexp: true,
    jsonc: true,
    yaml: true,
    toml: true,
    markdown: true,
    test: true,
  },
  {
    rules: {
      // 'unicorn/filename-case': [
      //   'error',
      //   {
      //     case: 'kebabCase',
      //     ignore: [
      //       'README.md',
      //       'App.vue',
      //       'src/components/ui/**',
      //     ],
      //   },
      // ],
      'perfectionist/sort-imports': 'off',
      'perfectionist/sort-exports': [
        'error',
        {
          type: 'natural',
        },
      ],
      'perfectionist/sort-named-imports': 'off',
      'perfectionist/sort-named-exports': [
        'error',
        {
          type: 'natural',
        },
      ],
      'jsdoc/check-param-names': 'off',
      'regexp/no-unused-capturing-group': 'off',
      'regexp/no-super-linear-backtracking': 'off',
    },
  },
);
