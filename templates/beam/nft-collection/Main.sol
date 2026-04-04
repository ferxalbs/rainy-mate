// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

contract BeamCollection {
    error NotOwner();
    error ZeroAddress();
    error TokenMissing();
    error Unauthorized();

    string public constant name = "Beam Collection";
    string public constant symbol = "BMC";

    address public owner;
    string public baseURI;
    uint256 public totalSupply;

    mapping(uint256 => address) private _ownerOf;
    mapping(address => uint256) private _balanceOf;
    mapping(uint256 => address) public getApproved;
    mapping(address => mapping(address => bool)) public isApprovedForAll;

    event Transfer(address indexed from, address indexed to, uint256 indexed tokenId);
    event Approval(address indexed owner, address indexed spender, uint256 indexed tokenId);
    event ApprovalForAll(address indexed owner, address indexed operator, bool approved);

    constructor() {
        owner = msg.sender;
        baseURI = "ipfs://beam-collection/";
    }

    function ownerOf(uint256 tokenId) public view returns (address) {
        address tokenOwner = _ownerOf[tokenId];
        if (tokenOwner == address(0)) revert TokenMissing();
        return tokenOwner;
    }

    function balanceOf(address account) external view returns (uint256) {
        if (account == address(0)) revert ZeroAddress();
        return _balanceOf[account];
    }

    function approve(address spender, uint256 tokenId) external {
        address tokenOwner = ownerOf(tokenId);
        if (msg.sender != tokenOwner && !isApprovedForAll[tokenOwner][msg.sender]) {
            revert Unauthorized();
        }
        getApproved[tokenId] = spender;
        emit Approval(tokenOwner, spender, tokenId);
    }

    function setApprovalForAll(address operator, bool approved) external {
        isApprovedForAll[msg.sender][operator] = approved;
        emit ApprovalForAll(msg.sender, operator, approved);
    }

    function transferFrom(address from, address to, uint256 tokenId) public {
        address tokenOwner = ownerOf(tokenId);
        if (tokenOwner != from) revert Unauthorized();
        if (to == address(0)) revert ZeroAddress();
        if (
            msg.sender != tokenOwner &&
            msg.sender != getApproved[tokenId] &&
            !isApprovedForAll[tokenOwner][msg.sender]
        ) revert Unauthorized();

        unchecked {
            _balanceOf[from] -= 1;
            _balanceOf[to] += 1;
        }
        _ownerOf[tokenId] = to;
        delete getApproved[tokenId];
        emit Transfer(from, to, tokenId);
    }

    function mint(address to) external onlyOwner returns (uint256 tokenId) {
        if (to == address(0)) revert ZeroAddress();
        tokenId = ++totalSupply;
        _ownerOf[tokenId] = to;
        unchecked {
            _balanceOf[to] += 1;
        }
        emit Transfer(address(0), to, tokenId);
    }

    function setBaseURI(string calldata nextBaseURI) external onlyOwner {
        baseURI = nextBaseURI;
    }

    function tokenURI(uint256 tokenId) external view returns (string memory) {
        ownerOf(tokenId);
        return string.concat(baseURI, _toString(tokenId));
    }

    function _toString(uint256 value) internal pure returns (string memory) {
        if (value == 0) return "0";
        uint256 digits;
        uint256 cursor = value;
        while (cursor != 0) {
            digits++;
            cursor /= 10;
        }
        bytes memory buffer = new bytes(digits);
        while (value != 0) {
            digits -= 1;
            buffer[digits] = bytes1(uint8(48 + uint256(value % 10)));
            value /= 10;
        }
        return string(buffer);
    }

    modifier onlyOwner() {
        if (msg.sender != owner) revert NotOwner();
        _;
    }
}
