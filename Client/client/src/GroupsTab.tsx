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


    }

    render(){
        return(
            <div className='groupTab'>

            </div>
        )
    }
}
