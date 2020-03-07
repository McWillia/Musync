import React, { Component } from "react";

interface IProps {
    code: string,
    client: WebSocket
}

interface IState {

}

export default class Playlist extends Component<IProps, IState> {
    constructor(props: IProps) {
        super(props)
        this.sendRequest = this.sendRequest.bind(this);
    }
  sendRequest(){
      var {client, code} = this.props;
      // console.log(code);

    this.props.client.send(JSON.stringify({code: this.props.code, type: 'get_playlists'}));
  }

  render(){
    return(
      <button onClick={this.sendRequest}  >test</button>
    )
  }
}
