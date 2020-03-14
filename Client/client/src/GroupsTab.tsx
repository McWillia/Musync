import React, { Component } from "react";

interface IProps {
    code: string,
    client: WebSocket,
    groups: IGroup[]
}

interface IState {

}

export interface IGroup {
    advert: boolean,
    group_id: number,
    clients: [number, string][]
}

export default class GroupTab extends Component<IProps, IState> {
    constructor(props: IProps) {
        super(props);
        this.handleClick = this.handleClick.bind(this);
    }

    componentDidMount() {
        let {code, client} = this.props;

        console.log(client.readyState)

    }

    handleClick (id : number){
        let {code, client} = this.props;

        client.send(JSON.stringify({
            'message_type': 'JoinGroup',
            'id': id,
        }))
    }

    render(){
        let {code, client, groups} = this.props;
        //
        // let obj: IGroup[] = data.data;
        // console.log(data);
        // console.log(typeof data.data)

        // let data = groups.data || [];
        let out = groups.map((group: IGroup) =>{
            return (
                <div>
                    {group.group_id}
                    <button
                        onClick={() => this.handleClick(group.group_id)}
                        >
                        Join
                    </button>
                    {group.clients.map((client: [number, string]) => {
                        return <text>{client[1]},</text>
                    })}
                </div>
            )
        });


        return(
            <div className='groupTab'>
                {out}
            </div>
        )
    }
}
