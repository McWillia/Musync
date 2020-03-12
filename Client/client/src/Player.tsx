import React, { Component } from "react";

interface IProps {
    code: string,
    client: WebSocket
}

interface IState {
    is_playing:boolean
}



export default class Player extends Component<IProps, IState> {
    constructor(props: IProps) {
        super(props);
        this.state = {
            is_playing: true
        }
        this.pausePlay = this.pausePlay.bind(this);
    }

    pausePlay(){
        let {code, client} = this.props;

        if(this.state.is_playing){
            //pause
            client.send(JSON.stringify({
                'message_type': 'Pause',
            }))
            this.setState({is_playing:false})
        } else {
            //play
            client.send(JSON.stringify({
                'message_type': 'Play',
            }))


            this.setState({is_playing:true})
        }
    }

    render(){
        return(
            <div>
                <button onClick={() => {this.pausePlay()}}>Pause/Play</button>
            </div>
    )
  }
}
