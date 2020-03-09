import React, { Component } from "react";

interface IProps {
    code: string,
    client: WebSocket
}

interface IState {

}

export default class MutualPlaylists extends Component<IProps, IState> {
    constructor(props: IProps) {
      super(props);
      this.sendRequest = this.sendRequest.bind(this);
    }
  sendRequest(){
      var {client, code} = this.props;

      this.props.client.send(JSON.stringify({message_type: 'MakeMutualPlaylist'}));
  }

  render(){
    return(
      <button onClick={this.sendRequest}  >Mutual Playlists</button>
    )
  }
}
