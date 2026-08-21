export type ClientRole = 'host' | 'viewer';

export interface BaseMessage {
    type: string;
}

export interface RegisterHostMessage extends BaseMessage {
    type: 'register_host';
    deviceId: string;
}

export interface GetHostsMessage extends BaseMessage {
    type: 'get_hosts';
}

export interface HostsListMessage extends BaseMessage {
    type: 'hosts_list';
    hosts: Array<{ sessionId: string; deviceId: string }>;
}

export interface HostRegisteredMessage extends BaseMessage {
    type: 'host_registered';
    sessionId: string;
}

export interface JoinSessionMessage extends BaseMessage {
    type: 'join_session';
    sessionId: string;
}

export interface SdpOfferMessage extends BaseMessage {
    type: 'sdp_offer';
    sessionId: string;
    sdp: string;
}

export interface SdpAnswerMessage extends BaseMessage {
    type: 'sdp_answer';
    sessionId: string;
    sdp: string;
}

export interface IceCandidateMessage extends BaseMessage {
    type: 'ice_candidate';
    sessionId: string;
    candidate: string;
    sdpMid: string | null;
    sdpMLineIndex: number | null;
}

export interface ErrorMessage extends BaseMessage {
    type: 'error';
    message: string;
}

export type SignalingMessage = 
    | RegisterHostMessage 
    | HostRegisteredMessage
    | JoinSessionMessage 
    | SdpOfferMessage 
    | SdpAnswerMessage 
    | IceCandidateMessage 
    | ErrorMessage
    | GetHostsMessage
    | HostsListMessage;
