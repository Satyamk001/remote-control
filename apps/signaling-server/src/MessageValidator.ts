import { SignalingMessage } from './types';

export class MessageValidator {
    public static validate(rawMessage: string): SignalingMessage | null {
        try {
            const parsed = JSON.parse(rawMessage);
            if (!parsed || typeof parsed !== 'object' || typeof parsed.type !== 'string') {
                return null;
            }

            switch (parsed.type) {
                case 'register_host':
                    if (typeof parsed.deviceId === 'string') {
                        return parsed as SignalingMessage;
                    }
                    break;
                case 'get_hosts':
                    return parsed as SignalingMessage;
                case 'join_session':
                case 'sdp_offer':
                case 'sdp_answer':
                    if (typeof parsed.sessionId === 'string') {
                        return parsed as SignalingMessage;
                    }
                    break;
                case 'ice_candidate':
                    if (typeof parsed.sessionId === 'string' && typeof parsed.candidate === 'string') {
                        return parsed as SignalingMessage;
                    }
                    break;
                default:
                    return null;
            }
            return null;
        } catch (e) {
            return null;
        }
    }
}
