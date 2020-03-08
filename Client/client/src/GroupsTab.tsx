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
    id: number,
    clients: string[]
}

export default class GroupTab extends Component<IProps, IState> {
    constructor(props: IProps) {
        super(props);
        this.handleClick = this.handleClick.bind(this);
    }

    componentDidMount() {
        let {code, client} = this.props;

        console.log(client.readyState)
        client.send(JSON.stringify({
            'message_type': 'get_advertising_groups'
        }))


    }

    handleClick (id : number){
        let {code, client} = this.props;

        client.send(JSON.stringify({
            'message_type': 'join_group',
            'id': id,
            'code':code
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
                    {group.id}
                    <button
                        onClick={() => this.handleClick(group.id)}
                        >
                        Join
                    </button>
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
