import React, { Component } from "react";

interface IProps {
    code: string,
    client: WebSocket,
    groups: string|null
}

interface IState {

}

export default class GroupTab extends Component<IProps, IState> {
    constructor(props: IProps) {
        super(props);
    }

    componentDidMount() {
        let {code, client} = this.props;

        console.log(client.readyState)
        client.send(JSON.stringify({
            'type': 'get_advertising_groups'
        }))


    }

    render(){
        let {code, client, groups} = this.props;


        console.log(groups)

        return(
            <div className='groupTab'>
                {groups}
            </div>
        )
    }
}
