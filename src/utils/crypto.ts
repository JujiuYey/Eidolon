/**
 * 简单的 Base64 加密/解密工具
 * 注意：这不是真正的加密，仅用于混淆 localStorage 中的敏感信息
 * 防止直接在开发者工具中明文看到 API Key
 */

/**
 * 加密文本（Base64 编码）
 * @param text 要加密的文本
 * @returns 加密后的文本
 */
export function encrypt(text: string): string {
  if (!text) {
    return '';
  }
  try {
    // 使用 btoa 进行 Base64 编码
    // 为了处理中文等 Unicode 字符，先转换为 URI 编码
    return btoa(encodeURIComponent(text));
  } catch (error) {
    console.error('加密失败:', error);
    return text;
  }
}

/**
 * 解密文本（Base64 解码）
 * @param encryptedText 加密的文本
 * @returns 解密后的文本
 */
export function decrypt(encryptedText: string): string {
  if (!encryptedText) {
    return '';
  }
  try {
    // 使用 atob 进行 Base64 解码
    // 解码后再进行 URI 解码得到原始文本
    return decodeURIComponent(atob(encryptedText));
  } catch (error) {
    console.error('解密失败:', error);
    return encryptedText;
  }
}
