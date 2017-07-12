import React, { Component } from 'react';
import axios from 'axios';

class Paths extends Component {
  constructor(props) {
    super(props);
    this.state = {
      paths: [],
    }
  }
  componentDidMount() {
    let that = this;
    axios.get('/ds/paths/' + this.props.id).then(resp => {
      that.setState({
        paths: resp.data.paths
      });
    })
  }

  render() {
    return (
      <div>{this.state.paths}</div>
    );
  }
}

export default Paths;
