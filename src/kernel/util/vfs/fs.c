#include "fs.h"

fs_node_t* fs_root = 0; //filesystem root

uint32_t read_fs(fs_node_t* node, uint32_t offset, uint32_t size, uint8_t* buffer) {
	//does the node have a read callback?
	if (node->read) {
		return node->read(node, offset, size, buffer);
	}
	return 0;
}

uint32_t write_fs(fs_node_t* node, uint32_t offset, uint32_t size, uint8_t* buffer) {
	//does the node have a write callback?
	if (node->write) {
		return node->write(node, offset, size, buffer);
	}
	return 0;
}

void open_fs(fs_node_t* node, uint8_t read, uint8_t write) {
	//does the node have an open callback?
	if (node->open) {
		return node->open(node);
	}
}

void close_fs(fs_node_t* node) {
	//does the node have a close callback?
	if (node->close) {
		return node->close(node);
	}
}

struct dirent* readdir_fs(fs_node_t* node, uint32_t index) {
	//is the node a directory, and does it have a callback?
	if ((node->flags & 0x7) == FS_DIRECTORY && node->readdir) {
		return node->readdir(node, index);
	}
	return 0;
}

fs_node_t* finddir_fs(fs_node_t* node, char* name) {
	//is the node a directory, and does it have a callback?
	if ((node->flags & 0x7) == FS_DIRECTORY && node->finddir) {
		return node->finddir(node, name);
	}
	return 0;
}

FILE* fopen(const char* filename, const char* mode) {
	/*
	//first, try looking in the current directory
	fs_node_t* file = finddir_fs(current_dir, name);
	//if that didn't work, check root directory
	//*/
	fs_node_t* file = finddir_fs(fs_root, filename);
	if (!file) {
		printf_err("Couldn't find file %s", filename);
		return;
	}
	FILE* stream = (FILE*)kmalloc(sizeof(FILE));
	memset(stream, 0, sizeof(FILE));
	stream->node = file;
	stream->fpos = 0;
	return stream;
}

int fgetc(FILE* stream) {
	char ch;
	read_fs(stream->node, stream->fpos++, 1, &ch);
	return ch;
}
