import React, { Component } from "react";

interface IProps {
    playlist_data?: string,
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
        this.props.client.send(JSON.stringify({code: this.props.code, type: 'get_playlists'}));
    }

    openPlaylist(song_name: string){
        alert("You have opened" + song_name)
    }

  render(){
    var obj = [];

    if(this.props.playlist_data !=null) {

      var data = this.props.playlist_data;
      // console.log(data);
      if (data) {

          var actual_data = JSON.parse(data);
          // console.log(actual_data);
          //
          //
          if(actual_data){
              // console.log(actual_data.items);

              obj = actual_data.items.map((playlist: any) => {
                console.log(playlist)
                return(
                  <tr>
                    <td>
                    {playlist.name}</td>
                    <td>
                    <button onClick={() => {this.openPlaylist(playlist.name)}}>Open {playlist.name}</button>
                    </td>
                    <td>{playlist.images.length > 1 && playlist.images[2].url ? <img src={playlist.images[2].url} />: <div></div>}</td>
                    <br />
                  </tr>
                )
              })
//<td>{playlist.images[2].url}</td>
              // for(var i = 0; i < actual_data.items.length && actual_data.items; i++){
              //       obj.push(<div>{actual_data.items[i].name} <button onClick={() => {this.openPlaylist(actual_data.items[i].name)}}>Open {actual_data.items[i].name}</button><br /></div>);
              //     }
              }
      }

    } else{
      obj.push(<div></div>)
    }
//<th>Playlist image</th>
    return(

      <div>

        <button onClick={this.sendRequest}  >Show playlists</button>
          <table id='printTable'>
            <tbody>
              <tr>
          		  <th>Playlist Name</th>
          		  <th>Playlist link</th>
                <th>Playlist image</th>

          	  </tr>
          	  {obj}
            </tbody>
        </table>
      </div>
    )
  }
}
