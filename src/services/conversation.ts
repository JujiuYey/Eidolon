import { invoke } from '@tauri-apps/api/core';
import type { AgentMessage } from '@/types';

interface ConversationMessageInput {
  role: AgentMessage['role'];
  content: string;
}

interface ConversationReply {
  content: string;
}

export async function sendConversationMessage(messages: AgentMessage[]): Promise<ConversationReply> {
  const payload: ConversationMessageInput[] = messages.map(message => ({
    role: message.role,
    content: message.content,
  }));

  return invoke<ConversationReply>('send_conversation_message', {
    messages: payload,
  });
}
