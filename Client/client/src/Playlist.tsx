import React, { Component } from "react";

interface IProps {
    code: string,
    client: WebSocket
}

interface IState {

}

export default class Playlist extends Component<IProps, IState> {
  sendRequest(){
    props.client.send({code: code, type: 'get_playlists'});
  }

  render(){
    return(
      <button onClick=this.sendRequest()>
    )
  }
}
