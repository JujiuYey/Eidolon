// highlight.js 的单语言文件没有自带类型声明，这里统一补一个
declare module 'highlight.js/lib/languages/*' {
  const language: import('highlight.js').LanguageFn;
  export default language;
}
