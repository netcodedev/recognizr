import { getContext, setContext } from 'svelte';
import type { RecognizrAPI, GalleryPerson } from './api';

interface AppContext {
	api: RecognizrAPI;
	showMessage: (text: string, type?: 'success' | 'error' | 'info') => void;
	getGallery: () => GalleryPerson[];
	loadGallery: () => Promise<void>;
}

const APP_CONTEXT_KEY = Symbol('app');

export function setAppContext(context: AppContext) {
	setContext(APP_CONTEXT_KEY, context);
}

export function getAppContext(): AppContext {
	return getContext(APP_CONTEXT_KEY);
}
